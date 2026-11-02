use biver_core::repository::{
    CheckOutBranchResult, CheckOutVersionResult, CommitResult, CreateBranchResult,
};

pub trait BiverResultUnwrap {
    type Output;

    fn unwrap(self) -> Self::Output;
}

impl BiverResultUnwrap for CommitResult {
    type Output = ();

    fn unwrap(self) -> Self::Output {
        if !self.is_ok() {
            panic!("CommitResult was {:?}", self);
        }
    }
}

impl BiverResultUnwrap for CheckOutBranchResult {
    type Output = ();
    fn unwrap(self) -> Self::Output {
        if !self.is_ok() {
            panic!("CheckOutBranchResult was {:?}", self);
        }
    }
}

impl BiverResultUnwrap for CheckOutVersionResult {
    type Output = ();
    fn unwrap(self) -> Self::Output {
        if !self.is_ok() {
            panic!("CheckOutVersionResult was {:?}", self);
        }
    }
}

impl BiverResultUnwrap for CreateBranchResult {
    type Output = ();
    fn unwrap(self) -> Self::Output {
        if !self.is_ok() {
            panic!("CreateBranchResult was {:?}", self);
        }
    }
}
