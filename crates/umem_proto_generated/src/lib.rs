mod memory;

use rmcp::schemars;

pub mod generated {
    tonic::include_proto!("memory");
}
