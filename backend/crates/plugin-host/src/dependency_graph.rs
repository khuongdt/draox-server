use server_core::PluginId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// DAG (Directed Acyclic Graph) for plugin dependencies with topological sort
/// and cycle detection.
pub struct DependencyGraph {
    /// plugin -> set of plugins it depends on
    dependencies: HashMap<PluginId, HashSet<PluginId>>,
    /// plugin -> set of plugins that depend on it (reverse index)
    dependents: HashMap<PluginId, HashSet<PluginId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a plugin to the graph (with no dependencies initially).
    pub fn add_plugin(&mut self, id: PluginId) {
        self.dependencies.entry(id.clone()).or_default();
        self.dependents.entry(id).or_default();
    }

    /// Add a dependency: `plugin` depends on `dependency`.
    pub fn add_dependency(
        &mut self,
        plugin: &PluginId,
        dependency: &PluginId,
    ) -> Result<(), DependencyError> {
        if !self.dependencies.contains_key(plugin) {
            return Err(DependencyError::PluginNotFound(plugin.clone()));
        }
        if !self.dependencies.contains_key(dependency) {
            return Err(DependencyError::PluginNotFound(dependency.clone()));
        }

        // Check if adding this edge would create a cycle
        if self.would_create_cycle(plugin, dependency) {
            // Build the cycle path for the error
            let path = self.find_cycle_path(plugin, dependency);
            return Err(DependencyError::CycleDetected { path });
        }

        self.dependencies
            .get_mut(plugin)
            .unwrap()
            .insert(dependency.clone());
        self.dependents
            .get_mut(dependency)
            .unwrap()
            .insert(plugin.clone());

        Ok(())
    }

    /// Remove a plugin and all its edges.
    pub fn remove_plugin(&mut self, id: &PluginId) {
        // Remove this plugin from all dependents lists (plugins that depended on it)
        if let Some(deps) = self.dependencies.remove(id) {
            for dep in &deps {
                if let Some(rev) = self.dependents.get_mut(dep) {
                    rev.remove(id);
                }
            }
        }

        // Remove this plugin from all dependencies lists (plugins it was a dependency of)
        if let Some(rev_deps) = self.dependents.remove(id) {
            for rev in &rev_deps {
                if let Some(fwd) = self.dependencies.get_mut(rev) {
                    fwd.remove(id);
                }
            }
        }
    }

    /// Get direct dependencies of a plugin.
    pub fn dependencies_of(&self, id: &PluginId) -> HashSet<PluginId> {
        self.dependencies
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get direct dependents of a plugin (plugins that depend on it).
    pub fn dependents_of(&self, id: &PluginId) -> HashSet<PluginId> {
        self.dependents
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    /// Topological sort -- returns activation order (dependencies first).
    /// Returns Err if there's a cycle.
    pub fn activation_order(&self) -> Result<Vec<PluginId>, DependencyError> {
        // Kahn's algorithm (BFS-based topological sort)
        let mut in_degree: HashMap<PluginId, usize> = HashMap::new();

        // Initialize in-degree for all nodes
        for id in self.dependencies.keys() {
            in_degree.entry(id.clone()).or_insert(0);
        }

        // Calculate in-degrees
        for (plugin, deps) in &self.dependencies {
            // Ensure the plugin is in the map
            in_degree.entry(plugin.clone()).or_insert(0);
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Wait -- in-degree should be: for each node, how many nodes depend on it?
        // Actually in Kahn's algorithm for topological sort of dependencies:
        // We want activation order = dependencies first.
        // A depends on B means B must come before A.
        // So the "edge" for topological sort is B -> A (B before A).
        // in-degree of A = number of dependencies A has.

        // Recalculate: in-degree = number of dependencies each plugin has
        let mut in_degree: HashMap<PluginId, usize> = HashMap::new();
        for id in self.dependencies.keys() {
            in_degree.insert(id.clone(), 0);
        }
        for (plugin, deps) in &self.dependencies {
            in_degree.insert(plugin.clone(), deps.len());
        }

        // Start with nodes that have no dependencies (in-degree 0)
        let mut queue: VecDeque<PluginId> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        // Sort the initial queue for deterministic order
        let mut sorted_initial: Vec<PluginId> = queue.drain(..).collect();
        sorted_initial.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        queue.extend(sorted_initial);

        let mut order = Vec::new();

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            // For each plugin that depends on this node, reduce its in-degree
            if let Some(rev_deps) = self.dependents.get(&node) {
                let mut sorted_deps: Vec<&PluginId> = rev_deps.iter().collect();
                sorted_deps.sort_by(|a, b| a.as_str().cmp(b.as_str()));

                for dependent in sorted_deps {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if order.len() != self.dependencies.len() {
            // Cycle detected -- find a node still in the graph with non-zero in-degree
            let remaining: Vec<PluginId> = in_degree
                .iter()
                .filter(|(_, deg)| **deg > 0)
                .map(|(id, _)| id.clone())
                .collect();
            Err(DependencyError::CycleDetected { path: remaining })
        } else {
            Ok(order)
        }
    }

    /// Detect if adding a dependency would create a cycle (DFS-based).
    /// Adding `from` depends on `to` means: from -> to edge in dependency graph.
    /// A cycle exists if `to` can already reach `from` through existing dependencies.
    fn would_create_cycle(&self, from: &PluginId, to: &PluginId) -> bool {
        // If from == to, it's a self-loop
        if from == to {
            return true;
        }

        // DFS from `to` following its dependencies to see if we can reach `from`
        let mut visited = HashSet::new();
        let mut stack = vec![to.clone()];

        while let Some(current) = stack.pop() {
            if current == *from {
                return true;
            }
            if visited.insert(current.clone()) {
                if let Some(deps) = self.dependencies.get(&current) {
                    for dep in deps {
                        if !visited.contains(dep) {
                            stack.push(dep.clone());
                        }
                    }
                }
            }
        }

        false
    }

    /// Build a cycle path for the error message.
    fn find_cycle_path(&self, from: &PluginId, to: &PluginId) -> Vec<PluginId> {
        // Find the path from `to` back to `from` through dependencies
        let mut path = Vec::new();
        let mut visited = HashSet::new();

        if self.dfs_path(to, from, &mut visited, &mut path) {
            // path is from `to` to `from`; prepend `from` and append `to` to show the full cycle
            let mut cycle = vec![from.clone()];
            cycle.push(to.clone());
            cycle.extend(path);
            cycle.push(from.clone());
            cycle
        } else {
            // Fallback: just show from -> to -> from
            vec![from.clone(), to.clone(), from.clone()]
        }
    }

    /// DFS to find a path from `current` to `target` through dependencies.
    fn dfs_path(
        &self,
        current: &PluginId,
        target: &PluginId,
        visited: &mut HashSet<PluginId>,
        path: &mut Vec<PluginId>,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            return false;
        }

        if let Some(deps) = self.dependencies.get(current) {
            for dep in deps {
                path.push(dep.clone());
                if self.dfs_path(dep, target, visited, path) {
                    return true;
                }
                path.pop();
            }
        }

        false
    }

    /// Check if a plugin can be safely deactivated (no active dependents).
    pub fn can_deactivate(&self, id: &PluginId, active_plugins: &HashSet<PluginId>) -> bool {
        let deps = self.dependents_of(id);
        // Can deactivate if none of the dependents are active
        !deps.iter().any(|dep| active_plugins.contains(dep))
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when working with the dependency graph.
#[derive(Debug, Clone)]
pub enum DependencyError {
    CycleDetected { path: Vec<PluginId> },
    PluginNotFound(PluginId),
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyError::CycleDetected { path } => {
                let names: Vec<String> = path.iter().map(|id| id.to_string()).collect();
                write!(f, "cycle detected: {}", names.join(" -> "))
            }
            DependencyError::PluginNotFound(id) => {
                write!(f, "plugin not found: {id}")
            }
        }
    }
}

impl std::error::Error for DependencyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(s: &str) -> PluginId {
        PluginId::from_str(s)
    }

    #[test]
    fn test_add_plugin() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));

        assert!(graph.dependencies_of(&pid("A")).is_empty());
        assert!(graph.dependencies_of(&pid("B")).is_empty());
        assert!(graph.dependents_of(&pid("A")).is_empty());
        assert!(graph.dependents_of(&pid("B")).is_empty());
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));

        // A depends on B
        graph.add_dependency(&pid("A"), &pid("B")).unwrap();

        assert!(graph.dependencies_of(&pid("A")).contains(&pid("B")));
        assert!(graph.dependents_of(&pid("B")).contains(&pid("A")));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));
        graph.add_plugin(pid("C"));

        // A -> B -> C -> A should fail at the last step
        graph.add_dependency(&pid("A"), &pid("B")).unwrap();
        graph.add_dependency(&pid("B"), &pid("C")).unwrap();

        let result = graph.add_dependency(&pid("C"), &pid("A"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DependencyError::CycleDetected { path } => {
                assert!(!path.is_empty());
            }
            other => panic!("expected CycleDetected, got: {other}"),
        }
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));
        graph.add_plugin(pid("C"));

        // A depends on B, B depends on C
        // Activation order should be: C, B, A (dependencies first)
        graph.add_dependency(&pid("A"), &pid("B")).unwrap();
        graph.add_dependency(&pid("B"), &pid("C")).unwrap();

        let order = graph.activation_order().unwrap();
        assert_eq!(order.len(), 3);

        let pos_a = order.iter().position(|x| x == &pid("A")).unwrap();
        let pos_b = order.iter().position(|x| x == &pid("B")).unwrap();
        let pos_c = order.iter().position(|x| x == &pid("C")).unwrap();

        // C must come before B, B must come before A
        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_can_deactivate() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));

        // A depends on B
        graph.add_dependency(&pid("A"), &pid("B")).unwrap();

        // B has active dependent A -- cannot deactivate B
        let active: HashSet<PluginId> = [pid("A"), pid("B")].into_iter().collect();
        assert!(!graph.can_deactivate(&pid("B"), &active));

        // A has no dependents -- can deactivate A
        assert!(graph.can_deactivate(&pid("A"), &active));

        // If A is not active, B can be deactivated
        let only_b: HashSet<PluginId> = [pid("B")].into_iter().collect();
        assert!(graph.can_deactivate(&pid("B"), &only_b));
    }

    #[test]
    fn test_remove_plugin() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));
        graph.add_plugin(pid("B"));
        graph.add_plugin(pid("C"));

        // A -> B -> C
        graph.add_dependency(&pid("A"), &pid("B")).unwrap();
        graph.add_dependency(&pid("B"), &pid("C")).unwrap();

        // Remove B
        graph.remove_plugin(&pid("B"));

        // A's dependencies should no longer include B
        assert!(!graph.dependencies_of(&pid("A")).contains(&pid("B")));
        // C's dependents should no longer include B
        assert!(!graph.dependents_of(&pid("C")).contains(&pid("B")));
        // B should be fully gone
        assert!(graph.dependencies_of(&pid("B")).is_empty());
        assert!(graph.dependents_of(&pid("B")).is_empty());
    }

    #[test]
    fn test_self_loop_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));

        let result = graph.add_dependency(&pid("A"), &pid("A"));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_not_found() {
        let mut graph = DependencyGraph::new();
        graph.add_plugin(pid("A"));

        let result = graph.add_dependency(&pid("A"), &pid("B"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DependencyError::PluginNotFound(id) => {
                assert_eq!(id, pid("B"));
            }
            other => panic!("expected PluginNotFound, got: {other}"),
        }
    }

    #[test]
    fn test_default_trait() {
        let graph = DependencyGraph::default();
        assert!(graph.activation_order().unwrap().is_empty());
    }

    #[test]
    fn test_dependency_error_display() {
        let err = DependencyError::PluginNotFound(pid("com.example.plugin"));
        assert!(err.to_string().contains("com.example.plugin"));

        let err = DependencyError::CycleDetected {
            path: vec![pid("A"), pid("B"), pid("A")],
        };
        assert!(err.to_string().contains("cycle detected"));
    }
}
