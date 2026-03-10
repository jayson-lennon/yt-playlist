use error_stack::Report;

use shownotes::cli::{run, RunError};

fn main() -> Result<(), Report<RunError>> {
    run()
}
