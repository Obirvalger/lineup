use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};

use crate::error::Error;

pub fn tsort<T: ToString, R: ToString, S: AsRef<str>>(
    graph: &BTreeMap<T, BTreeSet<R>>,
    place: S,
) -> Result<Vec<Vec<String>>> {
    let mut nodes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (node, edges) in graph.iter() {
        let edges = edges.iter().map(|v| v.to_string()).collect::<BTreeSet<String>>();
        nodes.insert(node.to_string(), edges);
    }
    let mut layers = vec![];
    while !nodes.is_empty() {
        let mut layer = vec![];
        for (node, edges) in nodes.iter() {
            if edges.is_empty() {
                layer.push(node.to_string());
            }
        }

        for node in layer.iter() {
            nodes.remove(node);
        }

        for (_, edges) in nodes.iter_mut() {
            for node in layer.iter() {
                edges.remove(node);
            }
        }

        if layer.is_empty() {
            bail!(Error::TSort(place.as_ref().to_string()));
        } else {
            layers.push(layer);
        }
    }

    Ok(layers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() -> Result<()> {
        let graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::from([]);
        let layers = tsort(&graph, "test")?;
        let expect: Vec<Vec<String>> = vec![];
        assert_eq!(layers, expect);

        Ok(())
    }

    #[test]
    fn edgeless_graph() -> Result<()> {
        let graph: BTreeMap<_, BTreeSet<&str>> = BTreeMap::from([
            ("A", BTreeSet::new()),
            ("B", BTreeSet::new()),
            ("C", BTreeSet::new()),
        ]);
        let layers = tsort(&graph, "test")?;
        let expect = vec![vec!["A", "B", "C"]];
        assert_eq!(layers, expect);

        Ok(())
    }

    #[test]
    fn chain_graph() -> Result<()> {
        let graph = BTreeMap::from([
            ("A", BTreeSet::new()),
            ("B", BTreeSet::from(["A"])),
            ("C", BTreeSet::from(["B"])),
        ]);
        let layers = tsort(&graph, "test")?;
        let expect = vec![vec!["A"], vec!["B"], vec!["C"]];
        assert_eq!(layers, expect);

        Ok(())
    }

    #[test]
    fn tree3() -> Result<()> {
        let graph = BTreeMap::from([
            ("A", BTreeSet::new()),
            ("B", BTreeSet::from(["A"])),
            ("C", BTreeSet::from(["A"])),
        ]);
        let layers = tsort(&graph, "test")?;
        let expect = vec![vec!["A"], vec!["B", "C"]];
        assert_eq!(layers, expect);

        Ok(())
    }

    #[test]
    fn tree4() -> Result<()> {
        let graph = BTreeMap::from([
            ("A", BTreeSet::new()),
            ("B", BTreeSet::from(["A"])),
            ("C", BTreeSet::from(["B"])),
            ("D", BTreeSet::from(["A"])),
        ]);
        let layers = tsort(&graph, "test")?;
        let expect = vec![vec!["A"], vec!["B", "D"], vec!["C"]];
        assert_eq!(layers, expect);

        Ok(())
    }

    #[test]
    fn diamond() -> Result<()> {
        let graph = BTreeMap::from([
            ("A", BTreeSet::new()),
            ("B", BTreeSet::from(["A"])),
            ("C", BTreeSet::from(["A"])),
            ("D", BTreeSet::from(["B", "C"])),
        ]);
        let layers = tsort(&graph, "test")?;
        let expect = vec![vec!["A"], vec!["B", "C"], vec!["D"]];
        assert_eq!(layers, expect);

        Ok(())
    }
}
