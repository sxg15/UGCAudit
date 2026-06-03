fn main() {
    if let Some(exit_code) = ugc_audit_lib::run_cli_from_args() {
        std::process::exit(exit_code);
    }
    ugc_audit_lib::detach_console_for_gui();
    ugc_audit_lib::run()
}
