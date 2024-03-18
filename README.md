# ForceAtlas2 Rust

Implementation of [ForceAtlas2](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4051631/) &#8211; force-directed Continuous Graph Layout Algorithm for Handy Network Visualization (i.e. position the nodes of a n-dimension graph for drawing it more human-readably)

![Example graph spacialized with ForceAtlas2-rs](https://txmn.tk/img/wot-fa2rs.png)

## Examples

[Install Rustup](https://rustup.rs/) and switch to nightly:

    rustup toolchain install nightly && rustup default nightly

Clone repository:

    git clone https://framagit.org/ZettaScript/forceatlas2-rs && cd forceatlas2-rs

The file `examples/wot.csv` lists the edges of a directed graph, in two columns.

### GTK viewer

Interactive viewer. You need GTK installed.

    cargo run --release -p viz -- examples/wot.csv

## Bindings

There is a binding for use in Python, [fa2rs](https://framagit.org/ZettaScript/fa2rs-py).

## Improvements

Previous versions of this crate were using hand-written AVX2 code to speed up repulsion by 16. It was however still O(N^2). Now the only implementation of repulsion uses the Barnes-Hut algorithm (quadtree/octree, O(NlogN)), which makes any kind of parallelization harder (SIMD, multithreading, GPU). Some research papers describe ways to parallelize quadtree building or even to implement it using SIMD or GPU.

## License

GNU AGPL v3, CopyLeft 2020-2024 Pascal Engélibert [(why copyleft?)](https://txmn.tk/blog/why-copyleft/)

Implementation details inspired by:
* [python-forceatlas2](https://code.launchpad.net/forceatlas2-python) (GNU GPL v3, CopyLeft 2016 Max Shinn)
* [python-fa2](https://github.com/bhargavchippada/forceatlas2) (GNU GPL v3, CopyLeft 2017 Bhargav Chippada)
* [Gephi](https://github.com/gephi/gephi/tree/master/modules/LayoutPlugin/src/main/java/org/gephi/layout/plugin/forceAtlas2) (GNU GPL v3 / CDDL 1.0, CopyLeft 2011 Gephi Consortium)
* [sigma.js](https://github.com/jacomyal/sigma.js/tree/master/plugins/sigma.layout.forceAtlas2), [Graphology](https://github.com/graphology/graphology-layout-forceatlas2/blob/master/iterate.js) (MIT, Guillaume Plique)
* [Anim-Wotmap](https://git.42l.fr/HugoTrentesaux/animwotmap) (Hugo Trentesaux)

The ForceAtlas2 paper was released under CC BY, Copyright 2014 Jacomy et al.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, version 3 of the License.  
This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.  
You should have received a copy of the GNU Affero General Public License along with this program. If not, see https://www.gnu.org/licenses/.
