#![allow(dead_code)]

use proptest::collection::vec;
use proptest::prelude::*;

prop_compose! {
    pub fn arb_versioned_file_content()(bytes in vec(any::<u8>(), 0..1024)) -> Vec<u8> {
        bytes
    }
}
