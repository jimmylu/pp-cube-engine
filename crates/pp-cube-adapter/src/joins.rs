//! The join graph: for the cubes of a query, the one tree of joins.
//!
//! Cube's own join graph tries each cube as the root and takes the shortest
//! path, choosing between candidates without a word. This one refuses where
//! that one would choose.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use cubenativeutils::CubeError;
use cubesqlplanner::cube_bridge::join_definition::{JoinDefinition, JoinDefinitionStatic};
use cubesqlplanner::cube_bridge::join_graph::JoinGraph;
use cubesqlplanner::cube_bridge::join_hints::JoinHintItem;
use cubesqlplanner::cube_bridge::join_item::{JoinItem, JoinItemStatic};
use cubesqlplanner::cube_bridge::join_item_definition::JoinItemDefinition;
use pp_cube_model::{Cube, Filter, Model, Query, Relationship};

use crate::schema::JoinDef;

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

pub(crate) struct Joins {
    model: Arc<Model>,
    /// The joins each cube declares, by the cube's position in the model.
    edges: Vec<Vec<Edge>>,
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
    /// Fails for a join to a cube the model does not declare.
    pub(crate) fn new(model: Arc<Model>) -> Result<Self, CubeError> {
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
            .collect::<Result<_, CubeError>>()?;
        Ok(Self { model, edges })
    }

    fn cube(&self, index: usize) -> &Cube {
        &self.model.cubes[index]
    }

    fn names(&self, cubes: &[usize]) -> String {
        quoted(cubes.iter().map(|cube| &self.cube(*cube).name))
    }

    fn relationship(&self, edge: Edge) -> Relationship {
        self.cube(edge.from).joins[edge.join].relationship
    }

    /// Fails unless the cubes `query` names form one tree of joins whose root
    /// owns every measure. A measure anywhere else needs the planner's
    /// multi-stage SQL, which is beyond the baseline.
    ///
    /// Call it after the members are known to be in the model.
    pub(crate) fn check(&self, query: &Query) -> Result<(), CubeError> {
        let mut members: Vec<&str> = Vec::new();
        members.extend(query.measures.iter().map(String::as_str));
        members.extend(query.dimensions.iter().map(String::as_str));
        members.extend(query.segments.iter().map(String::as_str));
        filter_members(&query.filters, &mut members);
        let time_dimensions = query.time_dimensions.iter();
        members.extend(time_dimensions.map(|time_dimension| time_dimension.dimension.as_str()));

        let mut cubes = Vec::new();
        let mut measures = Vec::new();
        for member in members {
            let Some((cube, name)) = member.split_once('.') else {
                continue;
            };
            let Some(cube) = self.model.cubes.iter().position(|known| known.name == cube) else {
                continue;
            };
            if !cubes.contains(&cube) {
                cubes.push(cube);
            }
            if self
                .cube(cube)
                .measures
                .iter()
                .any(|measure| measure.name == name)
            {
                measures.push((cube, member));
            }
        }

        let mut owners: Vec<usize> = measures.iter().map(|(cube, _)| *cube).collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() > 1 {
            return Err(CubeError::user(format!(
                "Measures of more than one cube ({}) are outside the baseline",
                self.names(&owners)
            )));
        }
        let Some(tree) = self.tree(&cubes)? else {
            return Err(self.unconnected(&cubes));
        };
        match measures.iter().find(|(cube, _)| *cube != tree.root) {
            None => Ok(()),
            Some((cube, measure)) => Err(CubeError::user(format!(
                "Measure `{measure}` belongs to `{}`, which this query reaches through a join \
                 from `{}`; measures are taken from the cube the joins start from",
                self.cube(*cube).name,
                self.cube(tree.root).name
            ))),
        }
    }

    fn unconnected(&self, cubes: &[usize]) -> CubeError {
        CubeError::user(format!(
            "No join path connects cubes {}: none of them reaches all the others",
            self.names(cubes)
        ))
    }

    /// The chains of joins that lead from `from` to `to`, as far as two: one
    /// is the answer, two are an ambiguity.
    fn paths(&self, from: usize, to: usize) -> Vec<Vec<Edge>> {
        let mut found = Vec::new();
        self.walk(from, to, &mut Vec::new(), &mut found);
        found
    }

    /// Extends `walked`, a chain of joins from `start`, towards `to` along
    /// every join that does not return to a cube already on the chain.
    fn walk(&self, start: usize, to: usize, walked: &mut Vec<Edge>, found: &mut Vec<Vec<Edge>>) {
        let at = walked.last().map_or(start, |step| step.to);
        if at == to {
            found.push(walked.clone());
            return;
        }
        for edge in &self.edges[at] {
            let visited = edge.to == start || walked.iter().any(|step| step.to == edge.to);
            if visited || found.len() > 1 {
                continue;
            }
            walked.push(*edge);
            self.walk(start, to, walked, found);
            walked.pop();
        }
    }

    /// The tree for `cubes`, or `None` when none of them reaches all the
    /// others. Fails when there is more than one way to join them, or when
    /// the joined cubes are not on one data source.
    fn tree(&self, cubes: &[usize]) -> Result<Option<Tree>, CubeError> {
        let mut trees = cubes.iter().filter_map(|root| {
            let others = cubes.iter().filter(|cube| *cube != root);
            let paths: Vec<_> = others
                .map(|cube| (*cube, self.paths(*root, *cube)))
                .collect();
            let reaches_all = paths.iter().all(|(_, paths)| !paths.is_empty());
            reaches_all.then_some((*root, paths))
        });
        let Some((root, paths)) = trees.next() else {
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
        for (cube, mut paths) in paths {
            if paths.len() > 1 {
                return Err(CubeError::user(format!(
                    "There is more than one join path from `{}` to `{}`",
                    self.cube(root).name,
                    self.cube(cube).name
                )));
            }
            for edge in paths.pop().unwrap_or_default() {
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
            }
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

    fn build(&self, cubes: &[usize]) -> Result<Option<Rc<dyn JoinDefinition>>, CubeError> {
        let Some(tree) = self.tree(cubes)? else {
            return Ok(None);
        };

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
