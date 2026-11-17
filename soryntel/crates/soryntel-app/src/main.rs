//! Soryntel Chemistry — CLI y servidor web local de ecuaciones de reacción.

mod cli;
mod composition;
mod http;

fn main() {
    cli::run();
}
