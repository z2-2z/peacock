use serde::{Serialize, Deserialize};
use libafl::prelude::{
    Input, HasTargetBytes, Error,
};
use libafl_bolts::prelude::{
    HasLen, OwnedSlice,
};
use ahash::RandomState;
use std::path::Path;
use std::fs::File;
use std::io::Read;

use crate::components::ffi::{
    generator_unparse,
    generator_serialize,
};

const BINARY_PREFIX: &str = "peacock-raw-";
static mut SERIALIZATION_BUFFER: [u8; 128 * 1024 * 1024] = [0; 128 * 1024 * 1024];

/// This component represents an Input during fuzzing.
#[derive(Serialize, Deserialize, Debug, Hash)]
pub struct PeacockInput {
    sequence: Vec<usize>,
}

impl PeacockInput {
    pub(crate) fn sequence(&self) -> &[usize] {
        &self.sequence
    }
    
    pub(crate) fn sequence_mut(&mut self) -> &mut Vec<usize> {
        &mut self.sequence
    }
}

impl Input for PeacockInput {
    fn generate_name(&self, _idx: usize) -> String {
        let hash = RandomState::with_seeds(0, 0, 0, 0).hash_one(self);
        format!("{}{:016x}", BINARY_PREFIX, hash)
    }
    
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut bytes: Vec<u8> = vec![];
        file.read_to_end(&mut bytes)?;
        
        let is_raw = if let Some(file_name) = path.file_name().and_then(|x| x.to_str()) {
            file_name.starts_with(BINARY_PREFIX)
        } else {
            false
        };
        
        if is_raw {
            Ok(postcard::from_bytes(&bytes)?)
        } else {
            let mut ret = Self::default();
            
            if !generator_unparse(&mut ret.sequence, &bytes) {
                return Err(Error::serialize(format!("Could not unparse sequence from input file {}", path.display())));
            }
            
            Ok(ret)
        }
    }
}

impl HasLen for PeacockInput {
    fn len(&self) -> usize {
        self.sequence.len()
    }
}

impl HasTargetBytes for PeacockInput {
    fn target_bytes(&self) -> OwnedSlice<u8> {
        let len = generator_serialize(&self.sequence, unsafe { &mut SERIALIZATION_BUFFER });
        
        unsafe {
            OwnedSlice::from_raw_parts(SERIALIZATION_BUFFER.as_ptr(), len)
        }
    }
}

impl Default for PeacockInput {
    fn default() -> Self {
        Self {
            sequence: Vec::with_capacity(4096 * 2),
        }
    }
}

impl Clone for PeacockInput {
    fn clone(&self) -> Self {
        let mut clone = Self::default();
        clone.sequence.extend_from_slice(&self.sequence);
        clone
    }
}
