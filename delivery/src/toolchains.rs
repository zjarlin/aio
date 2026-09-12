use az_plugin_delivery::BuildEnvironment;

pub fn settings(environment: BuildEnvironment) -> &'static [(&'static str, &'static str)] {
    match environment {
        BuildEnvironment::Rust => &[
            ("CARGO_HOME", "/cache/cargo"),
            ("CARGO_NET_GIT_FETCH_WITH_CLI", "false"),
            ("CARGO_UNSTABLE_GIT", "shallow-deps"),
            ("CARGO_HTTP_TIMEOUT", "30"),
            ("CARGO_NET_RETRY", "3"),
        ],
        BuildEnvironment::Kotlin => &[
            ("KOTLIN_CLI_NO_WELCOME_BANNER", "1"),
            ("KOTLIN_CLI_JAVA_HOME", "/opt/java/openjdk"),
            (
                "JAVA_TOOL_OPTIONS",
                "-Duser.home=/cache -XX:ActiveProcessorCount=4",
            ),
        ],
        BuildEnvironment::TypeScript => &[],
    }
}
