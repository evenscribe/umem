pub use rmcp::schemars;
mod memory;

pub mod generated {
    tonic::include_proto!("memory");
}

pub use generated::*;
