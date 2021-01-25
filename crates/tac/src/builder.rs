use crate::*;

pub struct FuncBuilder {
    func: TacFunc,
    cfg: petgraph::graphmap::DiGraphMap<usize, ()>,
    bb_count: usize,
    current_bb: usize,
    current_idx: Option<Index>,
}

impl FuncBuilder {
    pub fn new(name: SmolStr) -> FuncBuilder {
        let mut f = TacFunc::new(name);

        f.basic_blocks.insert(
            0,
            BasicBlock {
                op_start: None,
                op_end: None,
                params: None,
                jumps: Default::default(),
            },
        );

        FuncBuilder {
            func: f,
            cfg: petgraph::graphmap::DiGraphMap::new(),
            current_bb: 0,
            current_idx: None,
            bb_count: 0,
        }
    }

    pub fn current_bb(&self) -> usize {
        self.current_bb
    }

    pub fn new_bb(&mut self) -> usize {
        let bb_id = self.bb_count;
        self.bb_count += 1;
        self.cfg.add_node(bb_id);
        bb_id
    }

    pub fn set_current_bb(&mut self, bb_id: usize) -> TacResult<()> {
        let bb = self
            .func
            .basic_blocks
            .get(&bb_id)
            .ok_or(Error::NoSuchBB(bb_id))?;
        self.current_bb = bb_id;
        self.current_idx = bb.op_end;
        Ok(())
    }

    pub fn insert_after_current_place(&mut self, inst: Inst) -> Index {
        let idx = self.func.tac_new(inst);
        if let Some(cur_idx) = self.current_idx {
            self.func.tac_set_after(cur_idx, idx).unwrap();
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            if bb.op_end == Some(cur_idx) {
                bb.op_end = Some(idx);
            }
        } else {
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            bb.op_start = Some(idx);
            bb.op_end = Some(idx);
        }
        self.current_idx = Some(idx);
        idx
    }

    pub fn build(self) -> TacFunc {
        self.func
    }

    pub fn insert_at_end_of(&mut self, inst: Inst, bb_id: usize) -> TacResult<Index> {
        let curr_bb = self.current_bb;
        let curr_idx = self.current_idx;
        self.set_current_bb(bb_id)?;
        let insert_pos = self.insert_after_current_place(inst);
        self.current_bb = curr_bb;
        self.current_idx = curr_idx;
        Ok(insert_pos)
    }

    pub fn set_jump_inst(&mut self, inst: JumpInst, bb_id: usize) -> TacResult<Option<JumpInst>> {
        let bb = self
            .func
            .basic_blocks
            .get_mut(&bb_id)
            .ok_or(Error::NoSuchBB(bb_id))?;

        for target in inst.iter() {
            self.cfg.add_edge(bb_id, target, ());
        }

        let orig = std::mem::replace(&mut bb.jumps, inst);
        Ok(match orig {
            JumpInst::Unreachable => None,
            a => Some(a),
        })
    }

    pub fn pred_of<'a>(&'a self, bb_id: usize) -> impl Iterator<Item = usize> + 'a {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Incoming)
    }

    pub fn succ_of<'a>(&'a self, bb_id: usize) -> impl Iterator<Item = usize> + 'a {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Outgoing)
    }
}
