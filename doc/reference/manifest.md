# Manifest
Lineup manifest consists of several sections, most of which are optional.
* [use](#Use) - Use items from other files;
* [vars](#Vars) - Set global variables;
* [networks](#Networks) - Define networks;
* [storages](#Storages) - Define storages;
* [workers](#Workers) - Define workers;
* [default](#Default) - Overwrite defaults;
* [tasklines](#Tasklines) - Define tasklines;
* [taskset](#Taskset) - Define taskset;
* [extend](#Extend) - Extend some other sections.


# Use
Add items from other files to the main manifest. It has two subsections: `vars`
and `tasklines`. They are represented as an array of tables with keys:
* `module` - Name of a [module](modules.md). If it starts with `/` or `.`, it
    is interpreted as a path;
* `prefix` - Add prefix to names. By default, use module or file name. To use
    items without a prefix, set it to an empty string `""`;
* `items` - Array of names of items used from the module. By default, use all items.

For short, just the module name could be used instead of a table.  To work
properly with `vars`, the `prefix` should contain only alphanumerals and `_`.
When getting the default prefix from a module name, `-` will be substituted
with `_`.

Example of using a variable `update` and all tasklines from modules `apt-get`
and `useradd`:
```toml
[use]
vars = [{ module = "apt-get", items = ["update"] }]
tasklines = ["apt-get", "useradd"]


[taskset.show]
shell.cmd = "echo {{ apt_get.update }}"
shell.stdout = { print = true }

[taskset.install]
run = "apt-get.install"
vars.packages = "ncdu"

[taskset.user]
run = "useradd"
vars.user = "user"
```

# Vars
This is a table with string keys - [var definition](#Var-definition) and values
of any type. [Templates](templates.md) in strings are processed.

For example, a bool variable `build` stores the user's response to a question:
```toml
[vars]
build = "{{ confirm(msg='Do you want to build package?', default=true) }}"
```

## Var definition
A variable is defined by a string containing the variable name with an optional
[type](#Var-type) suffix and [kind](#Var-kind) prefix.

## Var type
The variable type is written after the variable name, delimited by `:`.
There are possible types:
* `bool`, `b`;
* `number`, `n`;
* `u64`, `u`;
* `i64`, `i`;
* `f64`, `f`;
* `string`, `s`;
* `array`, `a`;
* `object`, `o`.

Type union can be created by writing several types separated by `|`.
For example, the [ensure](#Ensure-task) variable `packages` should be `array`
or `string` in the `install` taskline:
```toml
[[tasklines.install]]
ensure.vars = ["packages: array | string"]
```

## Var kind
The variable kind is written before the variable name, delimited by `%`.
There are possible kinds:
* `fs` - Store a variable on the filesystem (use template filter or
    function `fs` to read value);
* `json`, `j` - Decode json value from string;
* `raw`, `r` - Does not render templates in value;
* `yaml` - Decode yaml value from string.
Example of appending `-m` to the flags array:
```toml
vars."json % flags" = "{{ flags | concat(with='-m') | json }}"
```
Example of storing an empty array in a `fs` variable `fs_var`:
```toml
vars."fs % fs_var" = []
```

## Special variables
There is a list of special variables set by lineup:
* [item](#Items) - Current item;
* `manifest_dir` - The directory where manifest is located;
* [result](#Task-result) - Result of previously run task;
* [taskline](#Tasklines) - Name of the current taskline;
* [worker](#Workers) - Name of the current worker.

# Networks
Networks describe virtual networks for workers. It is a table with network
names as keys. Values are network structs. The struct is represented via a
table. Values are:
* [engine](#Network-engine) - Specify parameters of a concrete engine (e.g., incus);

## Network engine
There is one engine type:
* [incus](#Network-engine-incus);

### Network engine incus
Network for container engine incus.
Incus-specific options are:
* `address` - ipv4 address;
* `nat` - Bool value controlling the use of ipv4 nat.

Example of creating an incus network `lpt`:
```toml
[networks.lpt.engine.incus]
address = "192.168.30.1/24"
```


# Storages
Storages describe storage volumes for workers. It is a table with volume
names as keys. Values are storage structs. The struct is represented via a
table. Values are:
* [engine](#Storage-engine) - Specify parameters of a concrete engine (e.g., incus);
* [items](#Items) - Multiplier to create several storages.

## Storage engine
There is one engine type:
* [incus](#Storage-engine-incus);

### Storage engine incus
Storage for container engine incus.
Incus-specific options are:
* `pool` - Pool for storage creation;
* `copy` - Volume in the same pool used as origin.

Example of creating an incus storage `people` in the default pool:
```toml
[storages.people.engine.incus]
```


# Workers
Workers describe runners of tasks (e.g., containers or virtual machines).
It is a table with worker names as keys. Values are worker structs.
The struct is represented via a table. Keys are:
* [engine](#Engine) - Specify parameters of a concrete engine (e.g., docker container);
* [items](#Items) - Multiplier to create several workers;
* `table-by-item` - Table indexed by item value;
* `table-by-name` - Table indexed by name.


# Default
Overwrite defaults in this section. Currently, it only has worker defaults.
For example, to use an alt podman container:
```toml
[default]
[default.worker.engine.podman]
image = "alt"
```


# Tasklines
An array of [tasks](#Task) that are run sequentially.


# Taskline
A simple way to use the "default" taskline. Same as:
```toml
[tasklines.""]
```


# Taskset
A table with names as keys and [tasks](#Task) as values.
Tasks in a taskset are supposed to run concurrently.
To provide order, a `requires` array could be used.
For example, the task `build` needs to be run after the task `setup`:
```toml
[taskset.build]
requires = ["setup"]
# Other task parameters
```

Also, tasks in a taskset could specify workers to run on using the `workers`
array, which consists of regexes of worker names. By default, tasks run on all
workers.

Taskset tasks specify `provide-workers`. It is an array of workers that are
available in this task and all tasks it runs. It is useful for
[run taskset](#RunTaskset-task) tasks. By default, it is `[]`.

# Task
Tasks are defined by a [task type](#Task-types) and have some parameters:
* `condition` - A shell command running on the worker. The task does not run if
    this fails;
* [items](#Items) - Multiplier to create several tasks;
* `parallel` - A bool controlling whether to run items tasks in parallel;
* [vars](#Task-vars) - Set variables;
* `export-vars` - Array of variable names that should be passed through a taskile;
* `clean-vars` - If true, run task without previously defined variables;
* [try](#Task-try) - Try running the task several attempts if fails;
* `table` - Table.

## Task result
Every task sets a `result` variable, containing the result of the task running.
If the `result` variable is not set, it has a `null` value.

## Task types
There are several types of tasks:
* [break](#Break-task) - Break execution of a taskline;
* [debug](#Debug-task) - Show message with log debug;
* [dummy](#Dummy-task) - Do nothing;
* [ensure](#Ensure-task) - Ensure taskline could be run;
* [error](#Error-task) - Raise an error;
* [exec](#Exec-task) - Run a command from an args array;
* [file](#File-task) - Copy a file to the worker;
* [get](#Get-task) - Copy a file from the worker;
* [info](#Info-task) - Show message with log info;
* [run-lineup](#RunLineup-task) - Run a lineup manifest;
* [run-taskline](#RunTaskline-task) - Run a taskline from the file;
* [run-taskset](#RunTaskset-task) - Run a taskset from the file;
* [run](#Run-task) - Run a taskline;
* [shell](#Shell-task) - Run a command from a shell string;
* [special](#Special-task) - Specific tasks supported by some engines;
* [test](#Test-task) - An array of commands;
* [trace](#Trace-task) - Show message with log trace;
* [warn](#Warn-task) - Show message with log warn.

## Break task
Stops execution of a taskline with a name given in the `taskline` parameter. By
default, it breaks the most inner taskline. Returns a previous result by
default, otherwise, the result could be set via a `result` parameter.

**Return:** `result`.

Example of breaking taskline `break` before running a failing command:
```toml
[[tasklines.break]]
break = {}

[[tasklines.break]]
shell.cmd = "false"
```

## Debug task
It shows a message from the `msg` parameter with a log debug. Returns the
previous result by default, otherwise, the result could be set via the `result`
parameter.

**Return:** `result`.

Example of greeting the worker:
```toml
debug.msg = "Hello {{ worker }}!"
```

## Dummy task
The only parameter `result` specifies the return value. By default, it is the
previous `result`.

**Return:** `result`.

Example of saving fs var and returning unchanged result:
```toml
dummy = {}
vars."fs % fs_var" = "LiL"
```

## Ensure task
It has a field `vars` with an array of variable names. Check them to be set.

**Return:** `true`.

Example of ensuring two variables `user` and `vars.lil` are set:
```toml
ensure.vars = ["user", "vars.lil"]
```

## Error task

Raises an error with a message from the `msg` parameter. Exits the process with
a `1` code by default, otherwise, the return code could be set via the `code`
parameter. Showing backtrace could be disabled via `trace` boolean parameter.

**Return:** `result`.

Example of failing with a message `Number not found`, an exit code `3` and
witout backtrace:
```toml
error.msg = "Number not found"
error.code = 3
error.trace = false
```

## Exec task
Consists of an `args` array of strings representing a command and
[common command parameters](#Common-command-parameters).

**Return:** Output of running command. Control output processing by
[command parameters result](#Command-parameters-result).

Example of showing date in utc:
```toml
exec.args = ["date", "--utc"]
exec.stdout.print = true
```

## File task
A file task has several fields:
* `dst` - Destination path on the worker to store the file;
* `src` - Source path on the host to get the file;
* `content` - String with contents of the file;
* `chown` - Change owner of the file (runs a `chown` utility);
* `chmod` - Change permissions of the file (runs a `chmod` utility).

**Return:** `dst`.

Example of creating `/tmp/test-file` on the worker:
```toml
file.dst = "/tmp/test-file"
file.content = "Test"
```

## Get task
A get task has several fields:
* `src` - Source path on the worker to get the file;
* `dst` - Destination path on the host to store the file. By default, store the
    file in the same directory as the manifest located with a source file name.

**Return:** `dst`.

Example of getting `/etc/os-release` from the worker:
```toml
get.src = "/etc/os-release"
```

## Info task
It shows a message from the `msg` parameter with a log info. Returns the
previous result by default, otherwise, the result could be set via the `result`
parameter.

**Return:** `result`.

Example of greeting the worker:
```toml
info.msg = "Hello {{ worker }}!"
```

## RunLineup task
Run a lineup manifest. Fields:
* `manifest` - Path to the lineup manifest;
* `exists` - Perform an action when a worker exists, variants are
    `fail`, `ignore`, and `replace`.
* `clean` - Boolean value controlling clean;
* `vars` - Pass extra vars to the manifest.

**Return:** `null`.

Example of running lineup manifest `build/LM.toml`:
```toml
run-lineup.manifest = "build/LM.toml"
```

## Run task
Run a taskline from manifest tasklines.

**Return:** `result` of the last task in the taskline.

Example of a task installing `apt-repo` with `apt-get`:
```toml
[use]
tasklines = ["apt-get"]


[taskset.install]
run = "apt-get.install"
vars.packages = "apt-repo"
```

## RunTaskline task
Run a taskline from a file. Field:
* `taskline` - Name of the taskline (default is "");
* `module` - Name of a [module](modules.md). Or path to the file containing
    the taskline, if it starts with `/` or `.`, it is interpreted as a path.

**Return:** `result` of the last task in the taskline.

Example of a task installing `apt-repo` with `apt-get`:
```toml
run-taskline = { module = "apt-get", taskline = "install" }
vars.packages = ["apt-repo"]
```

## RunTaskset task
Run a taskset from a file. Field:
* `module` - Name of a [module](modules.md). Or path to the file containing
    the taskset, if it starts with `/` or `.`, it is interpreted as a path;
* `worker` - Describes [workers](#RunTaskset-task-worker) to run the taskset on.

**Return:** `null`.

Example of running a taskset from the file `./LM-ts.toml`:
```toml
[taskset.setup]
run-taskline.module = "./LM-ts.toml"
run-taskline.worker = "all"
provide-workers = ["worker"]
# If you have more than one worker, set this to run the taskset only once
workers = ["worker"]
```
See `provided-workers` in [taskset](#Taskset).

Example of running a taskset from the file `./LM-ts.toml` with renaming a
`main-worker` to a `taskset-worker`:
```toml
[taskset.setup]
run-taskline.module = "./LM-ts.toml"
run-taskline.worker.maps = [
    ["main-worker", "taskset-worker"]
]
provide-workers = ["main-worker"]
# If you have more than one worker, set this to run the taskset only once
workers = ["main-worker"]
```
See `provided-workers` in [taskset](#Taskset).

### RunTaskset task worker
It describes workers passed to the `run-taskset` task. Variants:
* `all` - Pass all workers;
* `names` - Pass workers with names in `names`;
* `map` - Pass renamed according to the `map` workers.

## Shell task
Consists of a `command` string with a shell command and
[common command parameters](#Common-command-parameters).

**Return:** Output of running command. Control output processing by
[command parameters result](#Command-parameters-result).

Example of running an echo command:
```toml
shell.command = "echo LiL"
shell.stdout.print = true
```


## Special task
There are several types of special tasks:
* [restart](#Special-restart-task) - Restart vm or container;
* [start](#Special-start-task) - Start vm or container;
* [stop](#Special-stop-task) - Stop vm or container.

**Return:** `null`.

### Special restart task
Example of restarting:
```toml
[[tasklines.setup]]
special.restart = {}
```

### Special start task
Example of starting:
```toml
[[tasklines.setup]]
special.start = {}
```

### Special stop task
Example of stoping:
```toml
[[tasklines.setup]]
special.stop = {}
```

## Test task
Run commands. Fails on the first failure command run with check. List of fields:
* `commands` - Contains an array of args or shell commands;
* `check` - Used to overwrite the default check value for commands.

**Return:** A boolean value that is true if all tests complete successfully.

Example of running two commands and printing their output:
```toml
test.commands = [
    { cmd = "echo lil", stdout = { print = true } },
    { cmd = "echo lal >&2", stderr = { print = true } },
]
```

Example of running several success or failure commands with check disabled
by default:
```toml
test.check = false
test.commands = [
    "true", # shell task
    ["false"], # exec task
    { cmd = "false" }, # shell task
    { args = ["true"] }, # exec task
    { cmd = "true", check = true }, # check only this command
]
```

## Trace task
It shows a message from the `msg` parameter with a log trace. Returns the
previous result by default, otherwise, the result could be set via the `result`
parameter.

**Return:** `result`.

Example of greeting the worker:
```toml
trace.msg = "Hello {{ worker }}!"
```

## Warn task
It shows a message from the `msg` parameter with a log warn. Returns the
previous result by default, otherwise, the result could be set via the `result`
parameter.

**Return:** `result`.

Example of greeting the worker:
```toml
warn.msg = "Hello {{ worker }}!"
```

## Common command parameters
Some common command parameters:
* `check` - Fails the task if the command fails;
* [result](#Command-parameters-result) - Specify result value;
* `stdin` - Pass a provided string to the command's stdin;
* `stdout` - [Command output](#Command-output) for stdout;
* `stderr` - [Command output](#Command-output) for stderr;
* `success-codes` - Array of return codes treated as successful termination;
* `success-matches` - [Matches](#Matches) that need to be matched for success;
* `failure-matches` - [Matches](#Matches) that match means failure.

### Command parameters result
Configure returned result. It has several fields:
* `lines` - Split output stream to an array of lines or return a string;
* `matched` - Return true if success-matches or failure-matches is matched;
* `return-code` - Return just rc if true;
* `stream` - Set stream `stdout`(by default) or `stderr`;
* `strip` - Strip trailing whitespace symbols.

### Command output
Controls the redirection of the command output. Fields:
* `log` - Log output with a provided level;
* `print` - Print output to stdout.

For example, print stdout and log with `trace` level:
```toml
shell.command = "echo LiL"
shell.stdout = { print = true, log = "trace" }
```

### Matches
It is a formula consisting of `and`, `or`, and `err-re`, `out-re`, `any-re`.

For example, `failure-matches` in a `shell` task:
```toml
shell.command = "ls LLM.toml"
shell.failure-matches = { or = [ { err-re = "LLM" }, { err-re = "toml" } ] }
```

## Task vars
It is a table as in [vars](#Vars) or a list of tables as in
[extend vars](#Extend-vars).

Example of creating a shell task in a taskline and setting a variable `target`
and a `path` variable, which uses `target`:
```toml
[[taskline]]
shell.cmd = "ls -l {{ path }}"
vars = [
    { target = "debug" },
    { path = "target/{{ target }}/lineup" },
]
```

## Task try
Run the task until it finishes successfully or runs out of attempts.
Parameters:
* `attempts` - Number of attemps to run task;
* `sleep` - Sleep some seconds after fail (`1` by default);
* `cleanup.task` - Task runngin after fail to cleanup.

Example of running `possibly-create-dir-mydir` with `4` attempts, sleeping
`0.5` seconds and removing `mydir` as cleanup action:
```toml
shell.cmd = "possibly-create-dir-mydir"
try.attempts = 4
try.sleep = 0.5
try.cleanup.task.shell.cmd = "rm -rf mydir"
```


# Engine
Most engines have base fields:
* `name` - Set name for container or vm (default is worker's name);
* `setup` - Switch turning on or off setting the engine up;
* `exists` - Set action performed when the engine exists, variants are
    `fail`, `ignore`, and `replace`.

There are several types of engines:
* [dbg](#Dbg-engine);
* [docker](#Docker-engine);
* [incus](#Incus-engine);
* [host](#Host-engine);
* [podman](#Podman-engine);
* [ssh](#Ssh-engine);
* [vml](#Vml-engine).


## Dbg engine
Engine used to debug tasks. Just print information about running tasks. Could
be setting any keys, all are ignored.

Example of debugging `vm` worker with `vml` keys:
```toml
#[workers.vm.engine.vml]
[workers.vm.engine.dbg]
image = "alt"
mem = "4G"
```


## Docker engine
Container engine using docker.
Docker-specific options are:
* `load` - Path to saved image tarball to load;
* `memory` - Amount of memory;
* `image` - Image;
* `user` - User.

Example of a docker worker with name `docker` and `alt` image:
```toml
[workers.docker]
[workers.docker.engine.docker]
image = "alt"
```


## Incus engine
Container engine using incus.
Incus-specific options are:
* `image` - Image;
* `copy` - Copy created incus container;
* `net` - Describe network;
* `nproc` - Number of processors;
* `memory` - Amount of memory;
* `hostname` - Hostname;
* `storages` - Table with volume names as keys and
    [storage](#Incus-engine-storage) values;
* `user` - User.

Example of an incus worker with name `incus` and `alt/Sisyphus` image:
```toml
[workers.incus.engine.incus]
image = "alt/Sisyphus"
```

### Incus engine storage
Storage options are:
* `pool` - Pool for storage creation;
* `path` - Path within the container where the volume will be mounted;
* `readonly` - Bool value specifying whether the storage should be used in
    readonly mode;
* `volume` - Overwrite volume from `storages` key.

Example of an incus worker with name `gyle` and `alt/Sisyphus` image with the
`people` storage in the default pool used in a readonly mode:
```toml
[workers.gyle.engine.incus]
image = "alt/Sisyphus"
storages.people.path = "/people"
storages.people.readonly = true
```


## Host engine
Basic engine running commands just on your host.

Example of host worker with name `host`:
```toml
[workers.host]
engine = "host"
```


## Podman engine
Container engine using podman.
Podman-specific options are:
* `load` - Path to saved image tarball to load;
* `memory` - Amount of memory;
* `image` - Image;
* `pod` - Pod;
* `user` - User.

Example of a podman worker with name `podman` and `alt` image:
```toml
[workers.podman]
[workers.podman.engine.podman]
image = "alt"
```


## Ssh engine
Ssh engine for remote execution.
Ssh-specific options are:
* `host` - ssh host;
* `port` - ssh port;
* `user` - ssh user;
* `key` - ssh key;
* `ssh-cmd` - ssh command (default to `["ssh"]`).

Example of an ssh worker with name `localhost`:
```toml
[workers.localhost.engine.ssh]
host = "127.0.0.1"
```


## Vml engine
Virtual machine engine using vml.
Vml-specific options are:
* `vml-bin` - Path to vml binary;
* `memory` - Amount of memory;
* `image` - Image;
* `net` - Describe network;
* `nproc` - Number of processors;
* `parent` - Create vm with specific parent;
* `user` - User.

Example of a vml worker with name `vm`, `alt` image, and 2 gigabytes of memory:
```toml
[workers.vm]
[workers.vm.engine.vml]
image = "alt"
mem = "2G"
```


# Items
Items are used to multiply [workers](#Worker-items), [storages](#Storage-items)
or [tasks](#Task-items). It sets a [template](templates.md) variable `item`,
which can be used in strings as `{{ item }}`. Items could be one of four forms:
1. Array of strings or integers:
   ```toml
   items = ["a", 2]
   ```
2. Sequence defined by `end`, `start`, and `step`:
   ```toml
   items = { start = 1, end = 6, step = 2 }
   ```
3. Elements of a JSON array or keys of a JSON object:
   ```toml
   items.json = "{{ ['a', 'b'] | json }}"
   ```
4. Elements of an array variable or keys of an object variable with a given name:
   ```toml
   items.var = "commands"
   ```
5. Shell command, run on the host, which stdout is split by newlines:
   ```toml
   items = { command = "ls -d /lib*" }
   ```

## Worker items
When used with workers, the item could be used in the worker's name. Example of
creating two podman workers `buildbot-master` and `buildbot-worker`:

```toml
[workers."buildbot-{{item}}"]
items = ["master", "worker"]
[workers."buildbot-{{item}}".engine.podman]
image = "alt:sisyphus"
pod = "lineup-bb"
```

## Storage items
When used with storages, the item could be used in the storage's name. Example of
creating two incus storages `tasks-origin` and `tasks-sequential`:

```toml
[storages.'tasks-{{ item }}']
items = ["origin", "sequential"]
```

## Task items
In tasks, items do not render in the task name. Instead, it creates several
tasks with the same name. It is a sort of loop task. By default, tasks run in
parallel.
```toml
shell.cmd = "echo {{ item }}"
shell.stdout = { print = true }
items = { start = 1, end = 6, step = 2 }
parallel = false
```


# Extend
This section provides additional functionality to some other sections.
Consists of:
* [vars](#Extend-vars).

## Extend vars
Field `maps` gets a list of tables containing variables. Every element of the
list can use previous variables in templates.

Example of setting a variable `target` and a `path` variable, which uses `target`:
```toml
[extend]
vars.maps = [
    { target = "debug" },
    { path = "target/{{ target }}/lineup" },
]
```
