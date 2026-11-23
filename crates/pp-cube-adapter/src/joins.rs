//! The join graph: for the cubes of a query, the one tree of joins.
//!
//! Cube's own join graph tries each cube as the root and takes the shortest
//! path, choosing between candidates without a word. This one refuses where
//! that one would choose.
//!
//! Like Cube, it joins the cubes a join condition reads before the join that
//! reads them. Measures may come from any cube: the planner aggregates each
//! cube's measures on its own and combines them, so every such cube needs its
//! own tree to the rest of the query.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::join_definition::{JoinDefinition, JoinDefinitionStatic};
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::join_item::{JoinItem, JoinItemStatic};
use cubesqlplanner::cube_bridge::join_item_definition::JoinItemDefinition;
use pp_cube_model::{Cube, Filter, MeasureType, Model, Query, Relationship, Target};

use crate::schema::JoinDef;
use crate::PlanError;

/// A join of the model: the cube that declares it and its place among that
/// cube's joins.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Edge {
    from: usize,
    join: usize,
    to: usize,
}

/// The joins of a query, in the order SQL writes them: each after the cube
/// it starts from.
struct Tree {
    root: usize,
    edges: Vec<Edge>,
}

/// How one cube is reached from another.
enum Chain {
    One(Vec<Edge>),
    Several,
}

pub(crate) struct Joins {
    model: Arc<Model>,
    /// The joins each cube declares, by the cube's position in the model.
    edges: Vec<Vec<Edge>>,
    // Upstream bridge traits can return only CubeError. Keep the first fatal
    // local validation error typed until finish(), without parsing upstream text.
    // This cell belongs to one synchronous plan() call, never a shared model.
    local_failure: RefCell<Option<PlanError>>,
}

/// What a query reads, found by following member expressions.
#[derive(Default)]
struct Read<'m> {
    /// Every member met, with its cube; one is followed once.
    members: Vec<(usize, &'m str)>,
    /// The cubes of those members, in the order they were met.
    cubes: Vec<usize>,
    /// The measures among them, with their cubes.
    measures: Vec<(usize, &'m str)>,
    /// Expressions still to follow, each with the cube it belongs to.
    pending: Vec<(usize, &'m str)>,
}

fn quoted(names: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let names: Vec<_> = names
        .into_iter()
        .map(|name| format!("`{}`", name.as_ref()))
        .collect();
    names.join(", ")
}

/// The member of every condition in `filters`, groups included.
fn filter_members<'q>(filters: &'q [Filter], members: &mut Vec<&'q str>) {
    for filter in filters {
        match filter {
            Filter::And(filters) | Filter::Or(filters) => filter_members(filters, members),
            Filter::Condition(condition) => members.push(&condition.member),
        }
    }
}

impl Joins {
    /// The join graph of `model` for compiling `query`. Fails for a join to
    /// a cube the model does not declare, and for a query whose cubes the
    /// baseline does not join; see `check`.
    ///
    /// Call it after the members `query` names are known to be in the model.
    pub(crate) fn new(model: Arc<Model>, query: &Query) -> Result<Self, PlanError> {
        let position = |name: &str| model.cubes.iter().position(|cube| cube.name == name);
        let edges = model
            .cubes
            .iter()
            .enumerate()
            .map(|(from, cube)| {
                let joins = cube.joins.iter().enumerate();
                joins
                    .map(|(join, declared)| {
                        let to = position(&declared.name).ok_or_else(|| {
                            CubeError::user(format!(
                                "Cube `{}` joins `{}`, which is not in the model",
                                cube.name, declared.name
                            ))
                        })?;
                        Ok(Edge { from, join, to })
                    })
                    .collect()
            })
            .collect::<Result<_, CubeError>>()
            .map_err(PlanError::local)?;
        let joins = Self {
            model,
            edges,
            local_failure: RefCell::new(None),
        };
        joins.check(query)?;
        Ok(joins)
    }

    fn cube(&self, index: usize) -> &Cube {
        &self.model.cubes[index]
    }

    fn position(&self, cube: &str) -> Option<usize> {
        self.model.cubes.iter().position(|known| known.name == cube)
    }

    fn names(&self, cubes: &[usize]) -> String {
        quoted(cubes.iter().map(|cube| &self.cube(*cube).name))
    }

    fn relationship(&self, edge: Edge) -> Relationship {
        self.cube(edge.from).joins[edge.join].relationship
    }

    /// Notes that the query reads the member `name` of `cube`, and queues the
    /// member's expressions to be followed.
    fn read<'m>(&'m self, cube: usize, name: &str, read: &mut Read<'m>) {
        let definition = self.cube(cube);
        let dimension = definition
            .dimensions
            .iter()
            .find(|found| found.name == name);
        let measure = definition.measures.iter().find(|found| found.name == name);
        let segment = definition.segments.iter().find(|found| found.name == name);
        let known = dimension
            .map(|dimension| dimension.name.as_str())
            .or(measure.map(|measure| measure.name.as_str()))
            .or(segment.map(|segment| segment.name.as_str()));
        let Some(known) = known else {
            return;
        };
        if read.members.contains(&(cube, known)) {
            return;
        }
        read.members.push((cube, known));
        if !read.cubes.contains(&cube) {
            read.cubes.push(cube);
        }
        let expressions = dimension
            .map(|dimension| &dimension.sql)
            .into_iter()
            .chain(segment.map(|segment| &segment.sql));
        read.pending
            .extend(expressions.map(|expression| (cube, expression.as_str())));
        if let Some(measure) = measure {
            read.measures.push((cube, known));
            let conditions = measure.filters.iter().map(|filter| &filter.sql);
            let expressions = measure.sql.iter().chain(conditions);
            read.pending
                .extend(expressions.map(|expression| (cube, expression.as_str())));
        }
    }

    /// Fails unless the planner can join what `query` reads.
    ///
    /// A fact is a cube whose measures the query names, in its measures or in a
    /// filter, together with the cubes those measures read. The planner
    /// aggregates each fact on its own over the dimensions, filters and segments
    /// of the query, so each fact needs one tree of joins to all of those. With
    /// one fact that is the single tree of a flat query.
    ///
    /// What a query reads is the members it names and the members their
    /// expressions refer to, in any cube. The planner can join further cubes
    /// on its own account, for the primary key a count counts for one; `build`
    /// holds those to the same rule.
    fn check(&self, query: &Query) -> Result<(), PlanError> {
        let mut named: Vec<&str> = Vec::new();
        named.extend(query.measures.iter().map(String::as_str));
        named.extend(query.dimensions.iter().map(String::as_str));
        named.extend(query.segments.iter().map(String::as_str));
        let mut filtered: Vec<&str> = Vec::new();
        filter_members(&query.filters, &mut filtered);
        named.extend(filtered.iter().copied());
        let time_dimensions = query.time_dimensions.iter();
        named.extend(time_dimensions.map(|time_dimension| time_dimension.dimension.as_str()));

        // What the query reads apart from its measures, and what each fact reads.
        let mut shared = Read::default();
        let mut facts: Vec<(usize, Read)> = Vec::new();
        for member in named {
            let Some((cube, name)) = member.split_once('.') else {
                continue;
            };
            let Some(cube) = self.position(cube) else {
                continue;
            };
            let is_measure = self.cube(cube).measures.iter().any(|m| m.name == name);
            if !is_measure {
                self.read(cube, name, &mut shared);
                continue;
            }
            let position = facts.iter().position(|(owner, _)| *owner == cube);
            let position = position.unwrap_or_else(|| {
                facts.push((cube, Read::default()));
                facts.len() - 1
            });
            self.read(cube, name, &mut facts[position].1);
        }
        self.follow(&mut shared).map_err(PlanError::local)?;
        for (_, fact) in &mut facts {
            self.follow(fact).map_err(PlanError::local)?;
            for cube in &shared.cubes {
                if !fact.cubes.contains(cube) {
                    fact.cubes.push(*cube);
                }
            }
        }
        if facts.is_empty()
            && self
                .tree(&shared.cubes)
                .map_err(PlanError::local)?
                .is_none()
        {
            return Err(PlanError::local(self.unconnected(&shared.cubes)));
        }
        for (_, fact) in &facts {
            let Some(tree) = self.tree(&fact.cubes).map_err(PlanError::local)? else {
                return Err(PlanError::local(self.unconnected(&fact.cubes)));
            };
            self.check_filtered_counts(&tree, fact, &filtered)?;
        }
        let mut together: Vec<usize> = Vec::new();
        for cube in facts.iter().flat_map(|(_, fact)| &fact.cubes) {
            if !together.contains(cube) {
                together.push(*cube);
            }
        }
        // Facts are combined in one statement even when no join connects them.
        self.one_source(&together).map_err(PlanError::local)?;
        // When one tree holds every fact the planner may aggregate them together,
        // so a count is also held to that tree. A tree that cannot be built is the
        // planner's to meet: it asks for this one only when it wants it.
        if let (true, Ok(Some(tree))) = (facts.len() > 1, self.tree(&together)) {
            for (_, fact) in &facts {
                self.check_filtered_counts(&tree, fact, &filtered)?;
            }
        }
        Ok(())
    }

    /// Fails for a filter on a count whose rows the joins of `tree` repeat. The
    /// planner selects such a count as a count of distinct keys but compares the
    /// count of repeated rows in the filter, as Cube does; the filter would then
    /// disagree with the column it is read against.
    fn check_filtered_counts(
        &self,
        tree: &Tree,
        fact: &Read,
        filtered: &[&str],
    ) -> Result<(), PlanError> {
        let multiplied = self.multiplied(tree);
        for member in filtered {
            let Some((cube, name)) = member.split_once('.') else {
                continue;
            };
            let Some(cube) = self.position(cube) else {
                continue;
            };
            if !fact
                .measures
                .iter()
                .any(|(owner, measure)| *owner == cube && *measure == name)
            {
                continue;
            }
            // The counts the filtered measure is computed from, itself included.
            let mut read = Read::default();
            self.read(cube, name, &mut read);
            self.follow(&mut read).map_err(PlanError::local)?;
            for (owner, measure) in &read.measures {
                let definition = self.cube(*owner);
                let counts = definition.measures.iter().any(|found| {
                    found.name == *measure && found.measure_type == MeasureType::Count
                });
                if counts && multiplied.get(owner).copied().unwrap_or(false) {
                    return Err(PlanError::Unsupported(format!(
                        "The filter on `{member}` compares the count `{}.{measure}`, and this \
                         query reaches `{}` through a many_to_one join; a filter on a count over \
                         repeated rows is not supported",
                        definition.name, definition.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn unconnected(&self, cubes: &[usize]) -> CubeError {
        CubeError::user(format!(
            "No join path connects cubes {}: none of them reaches all the others",
            self.names(cubes)
        ))
    }

    /// Follows the expressions queued in `read` until none is left: each
    /// member they refer to is noted, and its own expressions are queued.
    fn follow<'m>(&'m self, read: &mut Read<'m>) -> Result<(), CubeError> {
        while let Some((cube, expression)) = read.pending.pop() {
            let targets = self
                .model
                .resolve(self.cube(cube), expression)
                .map_err(|error| CubeError::user(error.to_string()))?;
            for target in targets {
                let (owner, name) = match target {
                    Target::Column { .. } => continue,
                    Target::Dimension(owner, dimension) => (owner, &dimension.name),
                    Target::Measure(owner, measure) => (owner, &measure.name),
                };
                if let Some(owner) = self.position(&owner.name) {
                    self.read(owner, name, read);
                }
            }
        }
        Ok(())
    }

    /// The cubes the condition of the join `edge` reads besides the two it
    /// joins, directly or through the expression of a member. The planner names
    /// such a cube in the `ON` clause, so it has to be joined first.
    fn condition_cubes(&self, edge: Edge) -> Result<Vec<usize>, CubeError> {
        let join = &self.cube(edge.from).joins[edge.join];
        let mut read = Read::default();
        read.pending.push((edge.from, join.sql.as_str()));
        self.follow(&mut read)?;
        let outside = read.cubes.into_iter();
        Ok(outside
            .filter(|cube| *cube != edge.from && *cube != edge.to)
            .collect())
    }

    /// Whether each cube of `tree` has its rows repeated: a `many_to_one` join
    /// multiplies the cube it leads to, and so every cube joined beyond that one.
    fn multiplied(&self, tree: &Tree) -> HashMap<usize, bool> {
        let mut multiplied = HashMap::from([(tree.root, false)]);
        // Edges can be ordered by what their conditions read, so settle by passes.
        for _ in 0..=tree.edges.len() {
            for edge in &tree.edges {
                if let Some(from) = multiplied.get(&edge.from).copied() {
                    let many = self.relationship(*edge) == Relationship::ManyToOne;
                    multiplied.insert(edge.to, from || many);
                }
            }
        }
        multiplied
    }

    /// A shortest chain of joins from `from` to `to` that does not use the
    /// join `without`, or `None` when `to` is not reached. One pass over the
    /// joins.
    fn reach(&self, from: usize, to: usize, without: Option<Edge>) -> Option<Vec<Edge>> {
        let mut arrived_by: Vec<Option<Edge>> = vec![None; self.edges.len()];
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            if at == to {
                let mut chain = Vec::new();
                let mut at = to;
                while let Some(edge) = arrived_by[at] {
                    chain.push(edge);
                    at = edge.from;
                }
                chain.reverse();
                return Some(chain);
            }
            for edge in &self.edges[at] {
                let seen = edge.to == from || arrived_by[edge.to].is_some();
                if seen || Some(*edge) == without {
                    continue;
                }
                arrived_by[edge.to] = Some(*edge);
                queue.push_back(edge.to);
            }
        }
        None
    }

    /// How `to` is reached from `from`: not at all, by one chain of joins, or
    /// by more than one.
    ///
    /// A second chain, differing in any join, lacks some join of the first:
    /// a chain holding them all would be the first. So there is another chain
    /// exactly when `to` is still reached with one of those joins left out,
    /// which takes one pass per join of the chain, however many chains the
    /// model holds.
    fn path(&self, from: usize, to: usize) -> Option<Chain> {
        let chain = self.reach(from, to, None)?;
        let mut joins = chain.iter();
        let another = joins.any(|edge| self.reach(from, to, Some(*edge)).is_some());
        Some(if another {
            Chain::Several
        } else {
            Chain::One(chain)
        })
    }

    /// The tree for `cubes`, or `None` when none of them reaches all the
    /// others. Cubes that the conditions of its joins read are added and joined
    /// before the join that reads them, as Cube does. Fails when there is more
    /// than one way to join them, when join conditions need each other, or when
    /// the joined cubes are not on one data source.
    fn tree(&self, cubes: &[usize]) -> Result<Option<Tree>, CubeError> {
        let mut cubes = cubes.to_vec();
        // Each pass either adds a cube or ends; there are only so many cubes.
        let (root, mut edges) = loop {
            let Some((root, edges)) = self.spanning(&cubes)? else {
                return Ok(None);
            };
            let mut added = false;
            for edge in &edges {
                for cube in self.condition_cubes(*edge)? {
                    if !cubes.contains(&cube) {
                        cubes.push(cube);
                        added = true;
                    }
                }
            }
            if !added {
                break (root, edges);
            }
        };

        // SQL order: a join comes after the cube it starts from and after every
        // cube its condition reads.
        let mut joined = vec![root];
        let mut ordered = Vec::with_capacity(edges.len());
        while !edges.is_empty() {
            let mut ready = None;
            for (index, edge) in edges.iter().enumerate() {
                let needs = self.condition_cubes(*edge)?;
                if joined.contains(&edge.from) && needs.iter().all(|cube| joined.contains(cube)) {
                    ready = Some(index);
                    break;
                }
            }
            let Some(index) = ready else {
                let waiting: Vec<usize> = edges.iter().map(|edge| edge.to).collect();
                return Err(CubeError::user(format!(
                    "The joins to {} cannot be ordered: a join condition reads a cube that is \
                     joined only after it",
                    self.names(&waiting)
                )));
            };
            let edge = edges.remove(index);
            joined.push(edge.to);
            ordered.push(edge);
        }

        self.one_source(&joined)?;
        Ok(Some(Tree {
            root,
            edges: ordered,
        }))
    }

    /// Fails unless `cubes` are on one data source: SQL is written for one.
    fn one_source(&self, cubes: &[usize]) -> Result<(), CubeError> {
        let mut sources: Vec<(&str, &str)> = Vec::new();
        for cube in cubes.iter().map(|cube| self.cube(*cube)) {
            if !sources
                .iter()
                .any(|(_, source)| *source == cube.data_source)
            {
                sources.push((&cube.name, &cube.data_source));
            }
        }
        if sources.len() > 1 {
            let described: Vec<_> = sources
                .iter()
                .map(|(cube, source)| format!("`{cube}` on `{source}`"))
                .collect();
            return Err(CubeError::user(format!(
                "Cubes on different data sources cannot be joined: {}",
                described.join(", ")
            )));
        }
        Ok(())
    }

    /// The root that reaches every cube of `cubes` and the joins of those
    /// chains, or `None` when no cube reaches all the others. Fails when there
    /// is more than one way to join them.
    fn spanning(&self, cubes: &[usize]) -> Result<Option<(usize, Vec<Edge>)>, CubeError> {
        let mut trees = cubes.iter().filter_map(|root| {
            let others = cubes.iter().filter(|cube| *cube != root);
            let chains: Option<Vec<_>> = others
                .map(|cube| Some((*cube, self.path(*root, *cube)?)))
                .collect();
            Some((*root, chains?))
        });
        let Some((root, chains)) = trees.next() else {
            return Ok(None);
        };
        if let Some((other, _)) = trees.next() {
            return Err(CubeError::user(format!(
                "Cubes {} can be joined starting from `{}` or from `{}`; \
                 there is more than one join path",
                self.names(cubes),
                self.cube(root).name,
                self.cube(other).name
            )));
        }
        let mut edges: Vec<Edge> = Vec::new();
        for (cube, chain) in chains {
            let Chain::One(chain) = chain else {
                return Err(CubeError::user(format!(
                    "There is more than one join path from `{}` to `{}`",
                    self.cube(root).name,
                    self.cube(cube).name
                )));
            };
            for edge in chain {
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
            }
        }
        Ok(Some((root, edges)))
    }

    /// The cubes the planner names, each once, by position in the model.
    fn named(&self, hints: &[JoinHintItem]) -> Result<Vec<usize>, CubeError> {
        let mut cubes = Vec::new();
        let names = hints.iter().flat_map(|hint| match hint {
            JoinHintItem::Single(cube) => std::slice::from_ref(cube),
            JoinHintItem::Vector(cubes) => cubes.as_slice(),
        });
        for name in names {
            let cube = self.model.cubes.iter().position(|cube| cube.name == *name);
            let cube =
                cube.ok_or_else(|| CubeError::user(format!("Cube `{name}` is not in the model")))?;
            if !cubes.contains(&cube) {
                cubes.push(cube);
            }
        }
        if cubes.is_empty() {
            return Err(CubeError::internal(
                "a join tree was asked for no cube".to_owned(),
            ));
        }
        Ok(cubes)
    }

    /// The tree for the cubes the planner names, or `None` when no join path
    /// covers them.
    pub(crate) fn try_build(
        &self,
        hints: &[JoinHintItem],
    ) -> Result<Option<Rc<dyn JoinDefinition>>, CubeError> {
        self.build(&self.named(hints)?)
    }

    pub(crate) fn finish<T>(
        &self,
        result: Result<T, CubeError>,
        query: &Query,
        stage: &str,
    ) -> Result<T, PlanError> {
        // Local bridge validation is fatal even if upstream wraps or swallows it.
        if let Some(error) = self.local_failure.borrow_mut().take() {
            return Err(error);
        }
        result.map_err(|error| PlanError::upstream(error, query, stage))
    }

    fn build(&self, cubes: &[usize]) -> Result<Option<Rc<dyn JoinDefinition>>, CubeError> {
        let result = self.build_checked(cubes);
        if let Err(error) = &result {
            let mut failure = self.local_failure.borrow_mut();
            if failure.is_none() {
                *failure = Some(match error.cause {
                    cubenativeutils::CubeErrorCauseType::User => {
                        PlanError::User(error.message.clone())
                    }
                    _ => PlanError::Internal(error.message.clone()),
                });
            }
        }
        result
    }

    fn build_checked(&self, cubes: &[usize]) -> Result<Option<Rc<dyn JoinDefinition>>, CubeError> {
        let Some(tree) = self.tree(cubes)? else {
            return Ok(None);
        };
        // These are the cubes the planner joins, whatever made it join them. It
        // aggregates a measure over repeated rows through a subquery of distinct keys.
        let multiplied = self.multiplied(&tree);
        let mut items: Vec<Rc<dyn JoinItem>> = Vec::with_capacity(tree.edges.len());
        for edge in &tree.edges {
            let (from, to) = (&self.cube(edge.from).name, &self.cube(edge.to).name);
            items.push(Rc::new(Item {
                statics: JoinItemStatic {
                    from: from.clone(),
                    to: to.clone(),
                    original_from: from.clone(),
                    original_to: to.clone(),
                },
                definition: Rc::new(JoinDef::new(&self.cube(edge.from).joins[edge.join])?),
            }));
        }
        let multiplication_factor = multiplied
            .into_iter()
            .map(|(cube, multiplied)| (self.cube(cube).name.clone(), multiplied))
            .collect();
        Ok(Some(Rc::new(Joined {
            statics: JoinDefinitionStatic {
                root: self.cube(tree.root).name.clone(),
                multiplication_factor,
            },
            items,
        })))
    }
}

impl JoinGraph for Joins {
    fn build_join(
        &self,
        cubes_to_join: Vec<JoinHintItem>,
    ) -> Result<Rc<dyn JoinDefinition>, CubeError> {
        let cubes = self.named(&cubes_to_join)?;
        self.build(&cubes)?.ok_or_else(|| self.unconnected(&cubes))
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct Joined {
    statics: JoinDefinitionStatic,
    items: Vec<Rc<dyn JoinItem>>,
}

impl JoinDefinition for Joined {
    fn static_data(&self) -> &JoinDefinitionStatic {
        &self.statics
    }

    fn joins(&self) -> Result<Vec<Rc<dyn JoinItem>>, CubeError> {
        Ok(self.items.clone())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct Item {
    statics: JoinItemStatic,
    definition: Rc<JoinDef>,
}

impl JoinItem for Item {
    fn static_data(&self) -> &JoinItemStatic {
        &self.statics
    }

    fn join(&self) -> Result<Rc<dyn JoinItemDefinition>, CubeError> {
        Ok(self.definition.clone())
    }

    fn as_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

// scenario: B0-RESULT-002
#[test]
fn local_failure_survives_upstream_wrapping_and_success() {
    let model = Arc::new(
        pp_cube_model::load_yaml(include_str!(
            "../../pp-cube-compile/tests/fixtures/retail.yaml"
        ))
        .unwrap(),
    );
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    // Use a real bridge rejection: `refunds` reaches `regions` directly and
    // through `stores`.
    let joins = Joins::new(model, &query).unwrap();
    let refunds = joins.position("refunds").unwrap();
    let regions = joins.position("regions").unwrap();
    for upstream_success in [false, true] {
        assert!(joins.build(&[refunds, regions]).is_err());
        let result = if upstream_success {
            Ok(())
        } else {
            Err(CubeError::internal("wrapped failure".into()))
        };
        assert!(
            matches!(joins.finish(result, &query, "planning"), Err(PlanError::User(message)) if message.contains("more than one join path"))
        );
    }
}

// scenario: B0-JOIN-008
#[test]
fn a_lookup_without_a_path_is_an_answer_and_only_a_refusal_is_kept() {
    // The planner asks speculatively through `try_build`: at the pinned commit every
    // call site treats `None` as "no path, try something else" and propagates an
    // error. So "no path" must not be remembered, and a refusal must be.
    let model = Arc::new(
        pp_cube_model::load_yaml(include_str!(
            "../../pp-cube-compile/tests/fixtures/retail.yaml"
        ))
        .unwrap(),
    );
    let query = Query {
        measures: vec!["sales.amount".into()],
        ..Query::default()
    };
    let joins = Joins::new(model, &query).unwrap();
    let cube = |name: &str| JoinHintItem::Single(name.to_owned());
    // `regions` and `receipts` are both reached from `sales`, and neither reaches the other.
    assert!(matches!(
        joins.try_build(&[cube("regions"), cube("receipts")]),
        Ok(None)
    ));
    assert!(joins.finish(Ok(()), &query, "planning").is_ok());
    // A path that exists is still answered after a miss.
    assert!(matches!(
        joins.try_build(&[cube("sales"), cube("receipts")]),
        Ok(Some(_))
    ));
    assert!(joins.finish(Ok(()), &query, "planning").is_ok());
    // An ambiguous path is refused, and the refusal outlives a later success.
    assert!(joins
        .try_build(&[cube("refunds"), cube("regions")])
        .is_err());
    assert!(matches!(
        joins.try_build(&[cube("sales"), cube("stores")]),
        Ok(Some(_))
    ));
    assert!(matches!(
        joins.finish(Ok(()), &query, "planning"),
        Err(PlanError::User(message)) if message.contains("more than one join path")
    ));
}
