// use std::io::Result;
// fn main() -> Result<()> {
//     prost_build::compile_protos(&["abi.proto"], &["src/pb"])?;
//     Ok(())
// }

use std::io::Result;
fn main() -> Result<()> {
    prost_build::Config::new()
        .out_dir("src/pb")
        .compile_protos(&["abi.proto"], &["."])?;
    Ok(())
}
