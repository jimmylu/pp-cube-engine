//! The join graph: for the cubes of a query, the one tree of joins.
//!
//! Cube's own join graph tries each cube as the root and takes the shortest
//! path, choosing between candidates without a word. This one refuses where
//! that one would choose.

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
use pp_cube_model::{Cube, Filter, Model, Query, Relationship, Target};

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
    /// The cube of the query's measures, if it has any. Every tree that
    /// holds this cube starts from it.
    measure_cube: Option<usize>,
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
        let mut joins = Self {
            model,
            edges,
            measure_cube: None,
            local_failure: RefCell::new(None),
        };
        joins.measure_cube = joins.check(query)?;
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

    /// Fails unless the cubes `query` reads form one tree of joins whose root
    /// owns every measure, and returns the cube of the measures. A measure
    /// anywhere else needs the planner's multi-stage SQL, which is beyond the
    /// baseline.
    ///
    /// What a query reads is the members it names and the members their
    /// expressions refer to, in any cube. The planner can join further cubes
    /// on its own account, for the primary key a count counts for one; `build`
    /// holds those to the same rule.
    fn check(&self, query: &Query) -> Result<Option<usize>, PlanError> {
        let mut named: Vec<&str> = Vec::new();
        named.extend(query.measures.iter().map(String::as_str));
        named.extend(query.dimensions.iter().map(String::as_str));
        named.extend(query.segments.iter().map(String::as_str));
        filter_members(&query.filters, &mut named);
        let time_dimensions = query.time_dimensions.iter();
        named.extend(time_dimensions.map(|time_dimension| time_dimension.dimension.as_str()));

        let mut read = Read::default();
        for member in named {
            if let Some((cube, name)) = member.split_once('.') {
                if let Some(cube) = self.position(cube) {
                    self.read(cube, name, &mut read);
                }
            }
        }
        self.follow(&mut read).map_err(PlanError::local)?;
        let Some(tree) = self.tree(&read.cubes).map_err(PlanError::local)? else {
            return Err(PlanError::local(self.unconnected(&read.cubes)));
        };

        let mut owners: Vec<usize> = read.measures.iter().map(|(cube, _)| *cube).collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() > 1 {
            return Err(PlanError::Unsupported(format!(
                "Measures of more than one cube ({}) are outside the baseline",
                self.names(&owners)
            )));
        }
        match read.measures.iter().find(|(cube, _)| *cube != tree.root) {
            None => Ok(owners.first().copied()),
            Some((cube, measure)) => Err(PlanError::User(format!(
                "Measure `{owner}.{measure}` belongs to `{owner}`, which this query reaches \
                 through a join from `{}`; measures are taken from the cube the joins start from",
                self.cube(tree.root).name,
                owner = self.cube(*cube).name
            ))),
        }
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

    /// Fails if the condition of the join `edge` reads a cube other than the
    /// two it joins, directly or through the expression of a member. The
    /// planner would name that cube in the `ON` clause and not join it. Cube
    /// joins such a cube first; the baseline refuses the join.
    fn check_condition(&self, edge: Edge) -> Result<(), CubeError> {
        let declaring = self.cube(edge.from);
        let join = &declaring.joins[edge.join];
        let mut read = Read::default();
        read.pending.push((edge.from, join.sql.as_str()));
        self.follow(&mut read)?;
        let mut outside = read.cubes.iter();
        match outside.find(|cube| **cube != edge.from && **cube != edge.to) {
            None => Ok(()),
            Some(outside) => Err(CubeError::user(format!(
                "The join `{}` of cube `{}` reads `{}`, which is neither side of the join",
                join.name,
                declaring.name,
                self.cube(*outside).name
            ))),
        }
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
    /// others. Fails when there is more than one way to join them, when a
    /// join of the tree reads a cube it does not join, or when the joined
    /// cubes are not on one data source.
    fn tree(&self, cubes: &[usize]) -> Result<Option<Tree>, CubeError> {
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
        for edge in &edges {
            self.check_condition(*edge)?;
        }

        let joined = std::iter::once(root).chain(edges.iter().map(|edge| edge.to));
        let mut sources: Vec<(&str, &str)> = Vec::new();
        for cube in joined.map(|cube| self.cube(cube)) {
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
        Ok(Some(Tree { root, edges }))
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
        // These are the cubes the planner joins, whatever made it join them.
        if let Some(owner) = self.measure_cube.filter(|owner| cubes.contains(owner)) {
            if tree.root != owner {
                return Err(CubeError::user(format!(
                    "The measures of `{}` are reached through a join from `{}`; measures are \
                     taken from the cube the joins start from",
                    self.cube(owner).name,
                    self.cube(tree.root).name
                )));
            }
        }

        // Cube's rule: a `many_to_one` join multiplies the rows of the cube it
        // leads to, and so of every cube joined beyond that one.
        let mut multiplied = HashMap::from([(tree.root, false)]);
        let mut items: Vec<Rc<dyn JoinItem>> = Vec::with_capacity(tree.edges.len());
        for edge in &tree.edges {
            let many = self.relationship(*edge) == Relationship::ManyToOne;
            let from_multiplied = multiplied.get(&edge.from).copied().unwrap_or(false);
            multiplied.insert(edge.to, from_multiplied || many);
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
        measures: vec!["receipts.tax".into()],
        ..Query::default()
    };
    // Use the real bridge rejection, not a fabricated receipt.
    let joins = Joins::new(model, &query).unwrap();
    let sales = joins.position("sales").unwrap();
    let receipts = joins.position("receipts").unwrap();
    let mut joins = joins;
    joins.measure_cube = Some(receipts);
    for upstream_success in [false, true] {
        let _ = joins.build(&[sales, receipts]);
        let result = if upstream_success {
            Ok(())
        } else {
            Err(CubeError::internal("wrapped failure".into()))
        };
        assert!(
            matches!(joins.finish(result, &query, "planning"), Err(PlanError::User(message)) if message.contains("receipts"))
        );
    }
}
