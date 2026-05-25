High Risk Deny Policy narrows Permission Core runtime decisions with manifest-declared policy rules.

This child plugin mounts into `platform.permission-core` through `permission.policy`. It demonstrates the first runtime policy chain contract: policies can deny or warn after the built-in manifest, platform and consent checks have passed.

The sample denies dangerous `process.exec` targets and writes into system directories. It does not grant capabilities and does not own native execution.
