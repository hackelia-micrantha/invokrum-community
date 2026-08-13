#![forbid(unsafe_code)]

fn main() {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let code = invokrum_cli::run_with_stdin(
        std::env::args_os().skip(1),
        &mut stdin,
        &mut stdout,
        &mut stderr,
    );
    std::process::exit(code);
}
