use crate::*;

pub struct FuncBuilder {
    func: TacFunc,
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
                jumps: None,
            },
        );

        FuncBuilder {
            func: f,
            current_bb: 0,
            current_idx: None,
            bb_count: 0,
        }
    }

    pub fn new_bb(&mut self) -> usize {
        self.bb_count += 1;
        self.bb_count - 1
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
}
