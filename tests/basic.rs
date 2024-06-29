
#[cfg(test)]
mod test {
    use gtest::{Log, Program, System};
    const USER_ID: u64 = 100001;

    #[test]
    fn test_demo() {
        // Initialization of the common environment for running programs.
        let sys = System::new();

        // Initialization of the current program structure.
        let prog = Program::current(&sys);
        println!("{}", prog.id());
        // Send an init message to the program.
        let res = prog.send_bytes(USER_ID, b"Doesn't matter");

        // Check whether the program was initialized successfully.
        assert!(res.main_failed());

        // Send a handle message to the program.
        let res = prog.send_bytes(USER_ID, b"PING");

        // Check the result of the program execution.
        // 1. Create a log pattern with the expected result.
        let _log = Log::builder()
            .source(prog.id())
            .dest(USER_ID)
            .payload_bytes(b"PONG");

    }
}
