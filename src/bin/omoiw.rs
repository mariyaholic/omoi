#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() -> std::process::ExitCode {
    #[cfg(windows)]
    {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() != 3
            || args[0] != "--home"
            || args[2] != "collect"
            || !std::path::Path::new(&args[1]).is_absolute()
        {
            return std::process::ExitCode::from(2);
        }
        std::env::set_var("OMOI_HOME", &args[1]);
        std::process::ExitCode::from(omoi::cli::run_from(["omoi", "collect-scheduled"]))
    }
    #[cfg(not(windows))]
    {
        std::process::ExitCode::from(5)
    }
}
