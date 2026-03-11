# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-11)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* correct Basilica API types and SSH key support ([63d8174](https://github.com/PlatformNetwork/term-executor/commit/63d817449d1e3a19670d0138fd6a03e7d646a01f))
* enable apt/sudo in Basilica containers ([d83cb8c](https://github.com/PlatformNetwork/term-executor/commit/d83cb8c03e1484af2b66229742ff0853b0fecf22))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* full clone for commit checkout, explicit pip/pytest symlinks ([a0c1d6f](https://github.com/PlatformNetwork/term-executor/commit/a0c1d6fd20a69976f187bef3679388daa2f97991))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install base tools, runtimes, and filter redundant deps for Basilica ([80a3a0c](https://github.com/PlatformNetwork/term-executor/commit/80a3a0c86ee4e719732876b4801c6ee3d3ebbfb1))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* move workspace to /home/agent/sessions, fix node_modules permissions, improve agent code error handling ([1ced355](https://github.com/PlatformNetwork/term-executor/commit/1ced35517cd315e9f33c8c8dfb334d567a25a027))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* pip 22 compatibility for base tools and install commands ([68bb93f](https://github.com/PlatformNetwork/term-executor/commit/68bb93f7ae07456f5d9a52d4082697e9b5d9e097))
* remove redundant into_iter() for clippy ([eaf2a7c](https://github.com/PlatformNetwork/term-executor/commit/eaf2a7cfbe6f01cc4f0afcff7e8dcfc7da376ab2))
* report task status incrementally during batch execution ([4440fd8](https://github.com/PlatformNetwork/term-executor/commit/4440fd8a84502a04f2dd0462959380acb9c0b954))
* resolve all clippy warnings for CI ([2b3ae9d](https://github.com/PlatformNetwork/term-executor/commit/2b3ae9dfa2789c800cd8b2754b88aecc20bc4f21))
* revert Dockerfile git-lfs changes, add GIT_LFS_SKIP_SMUDGE to snapshot clone ([7130823](https://github.com/PlatformNetwork/term-executor/commit/71308239013109e0d415cb5a749563e3cdf13c99))
* run agent from repo_dir CWD, use absolute path to agent.py ([cc6bcde](https://github.com/PlatformNetwork/term-executor/commit/cc6bcde192c8afcdc700dd9c35ac228f174a7259))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add Basilica API client for container provisioning ([8a0afca](https://github.com/PlatformNetwork/term-executor/commit/8a0afcac73b0bbe3f45375336ac8b1d512f58685))
* add install field from swe-forge dataset, fix default split to train, add openssh-client ([737ab1f](https://github.com/PlatformNetwork/term-executor/commit/737ab1f24b3cdb50a3192437e5e9c6656ad2fb3e))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* auto-install language runtimes from install_config version fields ([25b2e94](https://github.com/PlatformNetwork/term-executor/commit/25b2e94511428b282cd43414c41964bdc9c4f26a))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* replace per-file HF downloads with bulk git clone snapshot ([6036b78](https://github.com/PlatformNetwork/term-executor/commit/6036b78ec19e2ba987d2e1d2e71890d8f731e5ca))
* run each task in its own Basilica container via SSH ([432107b](https://github.com/PlatformNetwork/term-executor/commit/432107b551629b4f073c594e8da3710ed2d6383d))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-11)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* correct Basilica API types and SSH key support ([63d8174](https://github.com/PlatformNetwork/term-executor/commit/63d817449d1e3a19670d0138fd6a03e7d646a01f))
* enable apt/sudo in Basilica containers ([d83cb8c](https://github.com/PlatformNetwork/term-executor/commit/d83cb8c03e1484af2b66229742ff0853b0fecf22))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* full clone for commit checkout, explicit pip/pytest symlinks ([a0c1d6f](https://github.com/PlatformNetwork/term-executor/commit/a0c1d6fd20a69976f187bef3679388daa2f97991))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install base tools, runtimes, and filter redundant deps for Basilica ([80a3a0c](https://github.com/PlatformNetwork/term-executor/commit/80a3a0c86ee4e719732876b4801c6ee3d3ebbfb1))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* move workspace to /home/agent/sessions, fix node_modules permissions, improve agent code error handling ([1ced355](https://github.com/PlatformNetwork/term-executor/commit/1ced35517cd315e9f33c8c8dfb334d567a25a027))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* pip 22 compatibility for base tools and install commands ([68bb93f](https://github.com/PlatformNetwork/term-executor/commit/68bb93f7ae07456f5d9a52d4082697e9b5d9e097))
* remove redundant into_iter() for clippy ([eaf2a7c](https://github.com/PlatformNetwork/term-executor/commit/eaf2a7cfbe6f01cc4f0afcff7e8dcfc7da376ab2))
* report task status incrementally during batch execution ([4440fd8](https://github.com/PlatformNetwork/term-executor/commit/4440fd8a84502a04f2dd0462959380acb9c0b954))
* resolve all clippy warnings for CI ([2b3ae9d](https://github.com/PlatformNetwork/term-executor/commit/2b3ae9dfa2789c800cd8b2754b88aecc20bc4f21))
* revert Dockerfile git-lfs changes, add GIT_LFS_SKIP_SMUDGE to snapshot clone ([7130823](https://github.com/PlatformNetwork/term-executor/commit/71308239013109e0d415cb5a749563e3cdf13c99))
* run agent from repo_dir CWD, use absolute path to agent.py ([cc6bcde](https://github.com/PlatformNetwork/term-executor/commit/cc6bcde192c8afcdc700dd9c35ac228f174a7259))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add Basilica API client for container provisioning ([8a0afca](https://github.com/PlatformNetwork/term-executor/commit/8a0afcac73b0bbe3f45375336ac8b1d512f58685))
* add install field from swe-forge dataset, fix default split to train, add openssh-client ([737ab1f](https://github.com/PlatformNetwork/term-executor/commit/737ab1f24b3cdb50a3192437e5e9c6656ad2fb3e))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* auto-install language runtimes from install_config version fields ([25b2e94](https://github.com/PlatformNetwork/term-executor/commit/25b2e94511428b282cd43414c41964bdc9c4f26a))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* replace per-file HF downloads with bulk git clone snapshot ([6036b78](https://github.com/PlatformNetwork/term-executor/commit/6036b78ec19e2ba987d2e1d2e71890d8f731e5ca))
* run each task in its own Basilica container via SSH ([432107b](https://github.com/PlatformNetwork/term-executor/commit/432107b551629b4f073c594e8da3710ed2d6383d))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-11)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* correct Basilica API types and SSH key support ([63d8174](https://github.com/PlatformNetwork/term-executor/commit/63d817449d1e3a19670d0138fd6a03e7d646a01f))
* enable apt/sudo in Basilica containers ([d83cb8c](https://github.com/PlatformNetwork/term-executor/commit/d83cb8c03e1484af2b66229742ff0853b0fecf22))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* full clone for commit checkout, explicit pip/pytest symlinks ([a0c1d6f](https://github.com/PlatformNetwork/term-executor/commit/a0c1d6fd20a69976f187bef3679388daa2f97991))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install base tools, runtimes, and filter redundant deps for Basilica ([80a3a0c](https://github.com/PlatformNetwork/term-executor/commit/80a3a0c86ee4e719732876b4801c6ee3d3ebbfb1))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* move workspace to /home/agent/sessions, fix node_modules permissions, improve agent code error handling ([1ced355](https://github.com/PlatformNetwork/term-executor/commit/1ced35517cd315e9f33c8c8dfb334d567a25a027))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* pip 22 compatibility for base tools and install commands ([68bb93f](https://github.com/PlatformNetwork/term-executor/commit/68bb93f7ae07456f5d9a52d4082697e9b5d9e097))
* report task status incrementally during batch execution ([4440fd8](https://github.com/PlatformNetwork/term-executor/commit/4440fd8a84502a04f2dd0462959380acb9c0b954))
* resolve all clippy warnings for CI ([2b3ae9d](https://github.com/PlatformNetwork/term-executor/commit/2b3ae9dfa2789c800cd8b2754b88aecc20bc4f21))
* run agent from repo_dir CWD, use absolute path to agent.py ([cc6bcde](https://github.com/PlatformNetwork/term-executor/commit/cc6bcde192c8afcdc700dd9c35ac228f174a7259))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add Basilica API client for container provisioning ([8a0afca](https://github.com/PlatformNetwork/term-executor/commit/8a0afcac73b0bbe3f45375336ac8b1d512f58685))
* add install field from swe-forge dataset, fix default split to train, add openssh-client ([737ab1f](https://github.com/PlatformNetwork/term-executor/commit/737ab1f24b3cdb50a3192437e5e9c6656ad2fb3e))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* auto-install language runtimes from install_config version fields ([25b2e94](https://github.com/PlatformNetwork/term-executor/commit/25b2e94511428b282cd43414c41964bdc9c4f26a))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* run each task in its own Basilica container via SSH ([432107b](https://github.com/PlatformNetwork/term-executor/commit/432107b551629b4f073c594e8da3710ed2d6383d))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-03)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* report task status incrementally during batch execution ([4440fd8](https://github.com/PlatformNetwork/term-executor/commit/4440fd8a84502a04f2dd0462959380acb9c0b954))
* run agent from repo_dir CWD, use absolute path to agent.py ([cc6bcde](https://github.com/PlatformNetwork/term-executor/commit/cc6bcde192c8afcdc700dd9c35ac228f174a7259))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run agent from repo_dir CWD, use absolute path to agent.py ([cc6bcde](https://github.com/PlatformNetwork/term-executor/commit/cc6bcde192c8afcdc700dd9c35ac228f174a7259))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* add --break-system-packages for pip installs + pip.conf bypass PEP 668 ([14430c4](https://github.com/PlatformNetwork/term-executor/commit/14430c4c5a991f0af3d77820a764ca2428d9dd4a))
* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* allow clippy too_many_arguments for run_task_pipeline ([6eb69c2](https://github.com/PlatformNetwork/term-executor/commit/6eb69c23050fedd89c86712bad32d2f6ac21c03b))
* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* extract full agent project instead of concatenating files ([3ac1023](https://github.com/PlatformNetwork/term-executor/commit/3ac1023c86246d3652ab9dbd8607979f37411b98))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* propagate agent_env to run_agent and pass --instruction arg to Python agents ([d922264](https://github.com/PlatformNetwork/term-executor/commit/d922264680f5e649c17f628b42a9bb379e36e746))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* upgrade Go to 1.23 and Node to 20 LTS in Dockerfile ([67ca713](https://github.com/PlatformNetwork/term-executor/commit/67ca713ff7497b89a003b75683a665543937ea25))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /code-hash endpoint for code integrity verification ([0a8e01b](https://github.com/PlatformNetwork/term-executor/commit/0a8e01b58d25732a73eb5017c8d54fb30472a80c))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* change default max_concurrent_tasks from 8 to 6, support CONCURRENTLY_TASKS env var ([eaba581](https://github.com/PlatformNetwork/term-executor/commit/eaba581ce21c153d3fce23bdeed5c13f1fefe269))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-03-02)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add /upload-agent-json endpoint for JSON-based agent upload ([9cfa1da](https://github.com/PlatformNetwork/term-executor/commit/9cfa1da9270b7e4d152c4d34200e2a3ff8a59f35))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-02-28)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* increase clone/install timeout from 180s to 600s ([95cecc3](https://github.com/PlatformNetwork/term-executor/commit/95cecc3f534f4aaa042e20f2c329ed91df237a31))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.3.0](https://github.com/PlatformNetwork/term-executor/compare/v2.2.0...v2.3.0) (2026-02-28)


### Bug Fixes

* auto-install deps, python3 symlink, detect full commands in fail_to_pass, language-aware test scripts ([a38497f](https://github.com/PlatformNetwork/term-executor/commit/a38497f58d004cf0c3325b576b4a0f64a7c108bc))
* config test race condition with env var mutex ([2963325](https://github.com/PlatformNetwork/term-executor/commit/2963325612199da06641704df0c71b37415bd745))
* expose agent_output and agent_patch in TaskResult and API responses ([348c251](https://github.com/PlatformNetwork/term-executor/commit/348c2512644a5ef8e23d76d91a42fed042c070b1))
* extract_agent_only for /evaluate - no tasks/ dir required ([2b90ee1](https://github.com/PlatformNetwork/term-executor/commit/2b90ee1d94439125e1df7c40864e77b6cf20eaf9))
* filter out apt-get/system commands from install (Basilica blocks syscalls), keep project-level installs ([e5365da](https://github.com/PlatformNetwork/term-executor/commit/e5365da581b661052b079fbe0dae6e4185bf0f7c))
* handle null test_patch from HuggingFace API (deserialize null as empty string) ([492d068](https://github.com/PlatformNetwork/term-executor/commit/492d06832ba463abf2d823067d7025353a900fa3))
* install corepack/yarn/pnpm globally via npm in Dockerfile ([b7183e8](https://github.com/PlatformNetwork/term-executor/commit/b7183e844299c31c28ea0a28ef38fd4b5eca0ed3))
* normalize repo URL in parse_task (add github.com prefix) ([398a6fd](https://github.com/PlatformNetwork/term-executor/commit/398a6fdba5167dbfd6353e10bac42157b8790701))
* run as root (Basilica blocks sudo), remove sudo prefix logic ([477a433](https://github.com/PlatformNetwork/term-executor/commit/477a43348d2c9afb8817e4c2727d5ff22f90f1da))
* sudo for apt-get in install commands, add golang/corepack/sudo to Dockerfile ([1aceb88](https://github.com/PlatformNetwork/term-executor/commit/1aceb88bf5ab97203819b4485d0cb7002c29269d))
* use :id path params for Axum 0.7 (not {id} which is 0.8) ([5dfa0c1](https://github.com/PlatformNetwork/term-executor/commit/5dfa0c1bbae4c2235198270e68e3bf8109f1368f))


### Features

* /evaluate endpoint using stored agent + TRUSTED_VALIDATORS whitelist ([b6aee7a](https://github.com/PlatformNetwork/term-executor/commit/b6aee7a49f107411ee33651141b44ac8263e3c71))
* add POST /submit_tasks endpoint + fix HuggingFace dataset compat ([d92444c](https://github.com/PlatformNetwork/term-executor/commit/d92444c1f9ddbc4b3502d949ac9fd3a381b9ada4))
* agent user with sudo for apt-install, run all commands as non-root agent ([e3f574a](https://github.com/PlatformNetwork/term-executor/commit/e3f574a700e2de142afc96f5ac2c9d6b525435fd))
* agent ZIP upload frontend with env vars + SUDO_PASSWORD auth ([3aa5184](https://github.com/PlatformNetwork/term-executor/commit/3aa518454755e35f855bc0c1779318e4a0149782))
* fat Docker image with all language runtimes (java, rust, pnpm, unzip, etc.) ([3855f2d](https://github.com/PlatformNetwork/term-executor/commit/3855f2d7bb83090d2744defda90c22c0ef20c78b))
* fetch task definitions from HF repo (workspace.yaml + tests/), remove auto_install hack ([7162a39](https://github.com/PlatformNetwork/term-executor/commit/7162a396d84025bc251bdeb291115e269479418f))
* swe-bench/swe-forge integration - extend WorkspaceConfig with fail_to_pass/pass_to_pass/install_config/difficulty fields - parse swe-forge workspace.yaml native fields as test script fallback - capture git diff (agent patch) after agent execution - add /dataset endpoint to fetch from HuggingFace CortexLM/swe-forge - wire fail_to_pass/pass_to_pass in dataset entry conversion ([814259e](https://github.com/PlatformNetwork/term-executor/commit/814259ea2d552fae81c6d1749701dc524782c8e2))

# [2.2.0](https://github.com/PlatformNetwork/term-executor/compare/v2.1.0...v2.2.0) (2026-02-20)


### Features

* **evaluation:** add evaluation module using platform-challenge-sdk types ([#6](https://github.com/PlatformNetwork/term-executor/issues/6)) ([78a369e](https://github.com/PlatformNetwork/term-executor/commit/78a369e46cfde69b8b748acf1215ec567cf5ae2c))

# [2.1.0](https://github.com/PlatformNetwork/term-executor/compare/v2.0.0...v2.1.0) (2026-02-20)


### Features

* integrate HuggingFace dataset handler with task/evaluation system ([db3ba95](https://github.com/PlatformNetwork/term-executor/commit/db3ba957c0f15cac899197e2e0455a8cf9ea39f9))

# [2.0.0](https://github.com/PlatformNetwork/term-executor/compare/v1.2.0...v2.0.0) (2026-02-18)


### Features

* **auth:** replace static hotkey/API-key auth with Bittensor validator whitelisting and 50% consensus ([#5](https://github.com/PlatformNetwork/term-executor/issues/5)) ([a573ad0](https://github.com/PlatformNetwork/term-executor/commit/a573ad04df1157843b8a825d24ed5c4df06f0f90))


### BREAKING CHANGES

* **auth:** WORKER_API_KEY env var and X-Api-Key header no longer required.
All validators on Bittensor netuid 100 with sufficient stake are auto-whitelisted.

* ci: trigger CI run

* fix(security): address auth bypass, input validation, and config issues

- Move nonce consumption AFTER signature verification in verify_request()
  to prevent attackers from burning legitimate nonces via invalid signatures
- Fix TOCTOU race in NonceStore::check_and_insert() using atomic DashMap
  entry API instead of separate contains_key + insert
- Add input length limits for auth headers (hotkey 128B, nonce 256B,
  signature 256B) to prevent memory exhaustion via oversized values
- Add consensus_threshold validation in Config::from_env() — must be
  in range (0.0, 1.0], panics at startup if invalid
- Add saturating conversion for consensus required calculation to prevent
  integer overflow on f64→usize cast
- Add tests for all security fixes

* fix(dead-code): remove orphaned default_concurrent fn and unnecessary allow(dead_code)

* fix: code quality issues in bittensor validator consensus

- Extract magic number 100 to configurable MAX_PENDING_CONSENSUS
- Restore #[allow(dead_code)] on DEFAULT_MAX_OUTPUT_BYTES constant
- Use anyhow::Context instead of map_err(anyhow::anyhow!) in validator_whitelist

* fix(security): address race condition, config panic, SS58 checksum, and container security

- consensus.rs: Fix TOCTOU race condition in record_vote by using
  DashMap entry API (remove_entry) to atomically check votes and remove
  entry while holding the shard lock, preventing concurrent threads from
  inserting votes between drop and remove
- config.rs: Replace assert! with proper Result<Self, String> return
  from Config::from_env() to avoid panicking in production on invalid
  CONSENSUS_THRESHOLD values
- main.rs: Update Config::from_env() call to handle Result with expect
- auth.rs: Add SS58 checksum verification using Blake2b-512 (correct
  Substrate algorithm) in ss58_to_public_key_bytes to reject addresses
  with corrupted checksums; previously only decoded base58 without
  validating the 2-byte checksum suffix
- Dockerfile: Add non-root executor user for container runtime security

* fix(dead-code): remove unused max_output_bytes config field and constant

Remove DEFAULT_MAX_OUTPUT_BYTES constant and max_output_bytes Config field
that were defined and populated from env but never read anywhere outside
config.rs. Both had #[allow(dead_code)] annotations suppressing warnings.

* fix(quality): replace expect/unwrap with proper error handling, extract magic numbers to constants

- main.rs: Replace .expect() on Config::from_env() with match + tracing::error! + process::exit(1)
- validator_whitelist.rs: Extract retry count (3) and backoff base (2) to named constants
- validator_whitelist.rs: Replace unwrap_or_else on Option with if-let pattern
- consensus.rs: Extract reaper interval (30s) to REAPER_INTERVAL_SECS constant

* fix(security): address multiple security vulnerabilities in PR files

- consensus.rs: Remove archive_data storage from PendingConsensus to
  prevent memory exhaustion (up to 50GB with 100 pending × 500MB each).
  Callers now use their own archive bytes since all votes for the same
  hash have identical data.

- handlers.rs: Stream multipart upload with per-chunk size enforcement
  instead of buffering entire archive before checking size limit.
  Sanitize error messages to not leak internal details (file paths,
  extraction errors) to clients; log details server-side instead.

- auth.rs: Add nonce format validation requiring non-empty printable
  ASCII characters (defense-in-depth against log injection and empty
  nonce edge cases).

- main.rs: Replace .unwrap() on TcpListener::bind and axum::serve with
  proper error logging and process::exit per AGENTS.md rules.

- ws.rs: Replace .unwrap() on serde_json::to_string with
  unwrap_or_default() to comply with AGENTS.md no-unwrap rule.

* fix(dead-code): rename misleading underscore-prefixed variable in consensus

* fix(quality): replace unwrap/expect with proper error handling in production code

- main.rs:21: Replace .parse().unwrap() on tracing directive with
  unwrap_or_else fallback to INFO level directive
- main.rs:36: Replace .expect() on workspace dir creation with
  error log + process::exit(1) pattern
- main.rs:110: Replace .expect() on ctrl_c handler with if-let-Err
  that logs and returns gracefully
- executor.rs:189: Replace semaphore.acquire().unwrap() with match
  that handles closed semaphore by creating a failed TaskResult

All changes follow AGENTS.md rule: no .unwrap()/.expect() in
production code paths. Test code is unchanged.

* docs: refresh AGENTS.md

# [1.2.0](https://github.com/PlatformNetwork/term-executor/compare/v1.1.0...v1.2.0) (2026-02-17)


### Features

* **auth:** add sr25519 signature + nonce verification ([dc8d8d4](https://github.com/PlatformNetwork/term-executor/commit/dc8d8d405e5e6d08100d900b8e94e29ced0b5417))
* **auth:** require API key alongside whitelisted hotkey ([#3](https://github.com/PlatformNetwork/term-executor/issues/3)) ([887f72b](https://github.com/PlatformNetwork/term-executor/commit/887f72bac8021e073e10d65e385ecb3205b55010))

# [1.1.0](https://github.com/PlatformNetwork/term-executor/compare/v1.0.0...v1.1.0) (2026-02-17)


### Features

* **executor:** add SWE-bench batch evaluation with hotkey auth and WebSocket streaming ([#2](https://github.com/PlatformNetwork/term-executor/issues/2)) ([8bfa8ee](https://github.com/PlatformNetwork/term-executor/commit/8bfa8eea464fc19b10eda23a834b3b019582d624))

# 1.0.0 (2026-02-17)


### Bug Fixes

* bump Rust Docker image to 1.85 for edition2024 support ([209f460](https://github.com/PlatformNetwork/term-executor/commit/209f460eb34788ff60604d2dc9c54c7c548be806))
* lowercase GHCR image tags for Docker push ([89449f9](https://github.com/PlatformNetwork/term-executor/commit/89449f992311c3712c5caa7a8d520dba09937866))
* remove target-cpu=native to avoid SIGILL on Blacksmith runners ([22bcb85](https://github.com/PlatformNetwork/term-executor/commit/22bcb85a7de818a03dcb43e01437316fd0ad0a0f))
* use rust:1.93-bookworm Docker image ([ddd1a24](https://github.com/PlatformNetwork/term-executor/commit/ddd1a2450e73bc41348a756b86dcb231d976acbd))


### Features

* initial term-executor — remote evaluation server for Basilica ([18f4f67](https://github.com/PlatformNetwork/term-executor/commit/18f4f673d213dc07522034346fdda656bd016352))
* production-ready implementation with Basilica integration ([5797025](https://github.com/PlatformNetwork/term-executor/commit/57970256c3a3201f6749f99933617a5e16fdd5cd))


### Performance Improvements

* minimal Docker image - remove all language runtimes from executor ([38058e8](https://github.com/PlatformNetwork/term-executor/commit/38058e8a848c0a945b411ea955eb56f0a9a5272a))
