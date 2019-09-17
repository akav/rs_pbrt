// std
use std;
use std::sync::Arc;
// pbrt
use crate::core::geometry::bnd3_union_bnd3;
use crate::core::geometry::{Bounds3f, Ray, Vector3f};
use crate::core::interaction::SurfaceInteraction;
use crate::core::light::AreaLight;
use crate::core::material::Material;
use crate::core::paramset::ParamSet;
use crate::core::pbrt::log_2_int_i32;
use crate::core::pbrt::Float;
use crate::core::primitive::Primitive;

pub struct KdAccelNode {}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum EdgeType {
    Start = 0,
    End = 1,
}

#[derive(Debug)]
pub struct BoundEdge {
    pub t: Float,
    pub prim_num: usize,
    pub edge_type: EdgeType,
}

impl BoundEdge {
    pub fn new(t: Float, prim_num: usize, starting: bool) -> Self {
        let edge_type: EdgeType;
        if starting {
            edge_type = EdgeType::Start;
        } else {
            edge_type = EdgeType::End;
        }
        BoundEdge {
            t,
            prim_num,
            edge_type,
        }
    }
}

impl Default for BoundEdge {
    fn default() -> Self {
        BoundEdge {
            t: 0.0 as Float,
            prim_num: 0_usize,
            edge_type: EdgeType::Start,
        }
    }
}

pub struct KdTreeAccel {
    pub isect_cost: i32,
    pub traversal_cost: i32,
    pub max_prims: i32,
    pub empty_bonus: Float,
    pub primitives: Vec<Arc<dyn Primitive + Sync + Send>>,
    pub nodes: Vec<KdAccelNode>,
    pub n_alloced_nodes: i32,
    pub next_free_node: i32,
    pub bounds: Bounds3f,
}

impl KdTreeAccel {
    pub fn new(
        p: Vec<Arc<dyn Primitive + Sync + Send>>,
        isect_cost: i32,
        traversal_cost: i32,
        empty_bonus: Float,
        max_prims: i32,
        max_depth: i32,
    ) -> Self {
        let p_len: usize = p.len();
        let mut max_depth: i32 = max_depth;
        let mut bounds: Bounds3f = Bounds3f::default();
        // build kd-tree for accelerator
        let n_alloced_nodes: i32 = 0;
        let next_free_node: i32 = 0;
        if max_depth <= 0 {
            max_depth =
                (8.0 as Float + 1.3 as Float * log_2_int_i32(p_len as i32) as Float).round() as i32;
        }
        // compute bounds for kd-tree construction
        let mut prim_bounds: Vec<Bounds3f> = Vec::with_capacity(p_len);
        for i in 0..p_len {
            let b: Bounds3f = p[i].world_bound();
            bounds = bnd3_union_bnd3(&bounds, &b);
            prim_bounds.push(b);
        }
        // allocate working memory for kd-tree construction
        let mut edges: [Vec<BoundEdge>; 3] = [
            Vec::with_capacity(2 * p_len),
            Vec::with_capacity(2 * p_len),
            Vec::with_capacity(2 * p_len),
        ];
        let mut prims0: Vec<usize> = Vec::with_capacity(p_len);
        let mut prims1: Vec<usize> = Vec::with_capacity((max_depth + 1) as usize * p_len);
        for i in 0..prims1.len() {
            prims1.push(0_usize);
        }
        // initialize _prim_nums_ for kd-tree construction
        let mut prim_nums: Vec<usize> = Vec::with_capacity(p_len);
        for i in 0..p_len {
            prims0.push(0_usize);
            prim_nums.push(i);
            // init all three edges Vecs
            edges[0].push(BoundEdge::default());
            edges[0].push(BoundEdge::default());
            edges[1].push(BoundEdge::default());
            edges[1].push(BoundEdge::default());
            edges[2].push(BoundEdge::default());
            edges[2].push(BoundEdge::default());
        }
        // start recursive construction of kd-tree
        let mut kd_tree: KdTreeAccel = KdTreeAccel {
            isect_cost,
            traversal_cost,
            max_prims,
            empty_bonus,
            primitives: p,
            nodes: Vec::new(),
            n_alloced_nodes,
            next_free_node,
            bounds,
        };
        // build_tree(0, bounds, prim_bounds, prim_nums.get(), primitives.size(),
        //           max_depth, edges, prims0.get(), prims1.get());
        KdTreeAccel::build_tree(
            &mut kd_tree,
            0 as i32,
            &bounds,
            &prim_bounds,
            &prim_nums[..],
            p_len,
            &mut edges,
            &mut prims0[..],
            &mut prims1[..],
            0, // bad_refines
        );
        kd_tree
    }
    pub fn create(prims: Vec<Arc<dyn Primitive + Send + Sync>>, ps: &ParamSet) -> Arc<KdTreeAccel> {
        let isect_cost: i32 = ps.find_one_int("intersectcost", 80);
        let trav_cost: i32 = ps.find_one_int("traversalcost", 1);
        let empty_bonus: Float = ps.find_one_float("emptybonus", 0.5 as Float);
        let max_prims: i32 = ps.find_one_int("maxprims", 1);
        let max_depth: i32 = ps.find_one_int("maxdepth", -1);
        Arc::new(KdTreeAccel::new(
            prims.clone(),
            isect_cost,
            trav_cost,
            empty_bonus,
            max_prims,
            max_depth,
        ))
    }
    pub fn build_tree(
        &mut self,
        node_num: i32,
        node_bounds: &Bounds3f,
        all_prim_bounds: &Vec<Bounds3f>,
        prim_nums: &[usize],
        n_primitives: usize,
        edges: &mut [Vec<BoundEdge>; 3],
        prims0: &mut [usize],
        prims1: &mut [usize],
        bad_refines: i32,
    ) {
        let mut bad_refines: i32 = bad_refines;
        assert_eq!(node_num, self.next_free_node);
        if self.next_free_node == self.n_alloced_nodes {}
        self.next_free_node += 1;
        // ...
        // choose split axis position for interior node
        let mut best_axis: i32 = -1;
        let mut best_offset: i32 = -1;
        let mut best_cost: Float = std::f32::INFINITY;
        let old_cost: Float = self.isect_cost as Float * n_primitives as Float;
        let total_sa: Float = node_bounds.surface_area();
        let inv_total_sa: Float = 1.0 as Float / total_sa;
        let d: Vector3f = node_bounds.p_max - node_bounds.p_min;
        // choose which axis to split along
        let mut axis: u8 = node_bounds.maximum_extent();
        let mut retries: u8 = 0;
        // avoid 'goto retrySplit;'
        loop {
            // initialize edges for _axis_
            for i in 0..n_primitives {
                let pn: usize = prim_nums[i];
                let bounds: &Bounds3f = &all_prim_bounds[pn];
                edges[axis as usize][2 * i] = BoundEdge::new(bounds.p_min[axis], pn, true);
                edges[axis as usize][2 * i + 1] = BoundEdge::new(bounds.p_max[axis], pn, false);
            }
            // sort _edges_ for _axis_
            edges[axis as usize].sort_unstable_by(|e0, e1| {
                if e0.t == e1.t {
                    e0.edge_type.partial_cmp(&e1.edge_type).unwrap()
                } else {
                    e0.t.partial_cmp(&e1.t).unwrap()
                }
            });
            // for i in 0..n_primitives {
            //     println!("{:?}", edges[axis as usize][2 * i]);
            //     println!("{:?}", edges[axis as usize][2 * i + 1]);
            // }

            // compute cost of all splits for _axis_ to find best
            let mut n_below: usize = 0;
            let mut n_above: usize = n_primitives;
            for i in 0..(2 * n_primitives) {
                if edges[axis as usize][i].edge_type == EdgeType::End {
                    n_above -= 1;
                }
                let edge_t: Float = edges[axis as usize][i].t;
                if edge_t > node_bounds.p_min[axis] && edge_t < node_bounds.p_max[axis] {
                    // compute cost for split at _i_th edge

                    // compute child surface areas for split at _edge_t_
                    let other_axis_0: u8 = (axis + 1) % 3;
                    let other_axis_1: u8 = (axis + 2) % 3;
                    let below_sa: Float = 2.0 as Float
                        * (d[other_axis_0] * d[other_axis_1]
                            + (edge_t - node_bounds.p_min[axis])
                                * (d[other_axis_0] + d[other_axis_1]));
                    let above_sa: Float = 2.0 as Float
                        * (d[other_axis_0] * d[other_axis_1]
                            + (node_bounds.p_max[axis] - edge_t)
                                * (d[other_axis_0] + d[other_axis_1]));
                    let p_below: Float = below_sa * inv_total_sa;
                    let p_above: Float = above_sa * inv_total_sa;
                    let eb: Float;
                    if n_above == 0 || n_below == 0 {
                        eb = self.empty_bonus;
                    } else {
                        eb = 0.0 as Float;
                    }
                    let cost: Float = self.traversal_cost as Float
                        + self.isect_cost as Float
                            * (1.0 as Float - eb)
                            * (p_below * n_below as Float + p_above * n_above as Float);
                    // update best split if this is lowest cost so far
                    if cost < best_cost {
                        best_cost = cost;
                        best_axis = axis as i32;
                        best_offset = i as i32;
                    }
                }
            }
            assert!(n_below == n_primitives && n_above == 0);
            // create leaf if no good splits were found
            if best_axis == -1 && retries < 2 {
                retries += 1;
                axis = (axis + 1) % 3;
            // goto retrySplit;
            } else {
                break;
            }
        }
        if best_cost > old_cost {
            bad_refines += 1;
        }
        if (best_cost > 4.0 as Float * old_cost && n_primitives < 16)
            || best_axis == -1
            || bad_refines == 3
        {
            // TODO: nodes[node_num].init_leaf(primNums, n_primitives, &primitiveIndices);
            return;
        }
        // classify primitives with respect to split
        let mut n0: usize = 0;
        let mut n1: usize = 0;
        for i in 0..best_offset as usize {
            if edges[best_axis as usize][i].edge_type == EdgeType::Start {
                prims0[n0] = edges[best_axis as usize][i].prim_num;
                n0 += 1;
            }
        }
        for i in ((best_offset + 1) as usize)..(2 * n_primitives) {
            if edges[best_axis as usize][i].edge_type == EdgeType::End {
                prims1[n1] = edges[best_axis as usize][i].prim_num;
                n1 += 1;
            }
        }
    }
}

impl Primitive for KdTreeAccel {
    fn world_bound(&self) -> Bounds3f {
        // WORK
        Bounds3f::default()
    }
    fn intersect(&self, ray: &mut Ray) -> Option<SurfaceInteraction> {
        // WORK
        None
    }
    fn intersect_p(&self, ray: &Ray) -> bool {
        // WORK
        false
    }
    fn get_material(&self) -> Option<Arc<dyn Material + Send + Sync>> {
        None
    }
    fn get_area_light(&self) -> Option<Arc<dyn AreaLight + Send + Sync>> {
        None
    }
}