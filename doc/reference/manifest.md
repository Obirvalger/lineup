# Manifest

Lineup manifest consists of several sections, most of which are optional.

* [use](#Use) - Use items from other files;
* [vars](#Vars) - Set global variables;
* [workers](#Workers) - Define workers;
* [default](#Default) - Overwrite defaults;
* [tasklines](#Tasklines) - Define tasklines;
* [taskset](#Taskset) Define taskset;
* [extend](#Extend) Extend some other sections.


# Use
Add items from other files to the main manifest. It has two subsections -
`vars` and `tasklines`. They are represented via array of tables with keys:
* `module` - Name of a [module](modules.md). Or path to the file containing
    the taskline, if it starts with `/` or `.`, it is interpreted as a path;
* `prefix` - Add prefix to names. By default use module or file name. To use
    items without prefix, set it to an empty string `""`;
* `items` - Array of names of items used from module. By default use all items.

For short just module name could be used instead of a table.
To work properly with `vars` the `prefix` should contain only alphanumerals
and `_`. When get default prefix from a module name, `-` will be substituted
with `_`.

Example of using a variable `update` and all tasklines from modules `apt-get`
and `useradd`:
```toml
[use]
vars = [{ module = "apt-get", items = ["update"]}]
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
This is a table with string keys - [var defenition](#var-defenition) and
values of any type. [Templates](templates.md) in strings are processed.
For example, a bool variable `build` stores the user's response to a question:
```toml
[vars]
build = "{{ confirm(msg='Do you want to build package?', default=true) }}"
```

## Var defenition
Variable defined by string containg vairable name with optional [type](#var-type) suffix
and [kind](#var-kind) prefix.

## Var type
Variable type writes after variable name delimited by `:`.
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
For example, [ensure](#ensure-task) variable `packages` should be `array` or
`string` in `install` taskline:
```toml
[[tasklines.install]]
ensure.vars = ["packages: array | string"]
```

## Var kind
Variable kind writes before variable name delimited by `%`.
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
Example of storing empty array to a `fs` vaiable `fs_var`:
```toml
vars."fs % fs_var" = []
```


# Workers
Workers describe runners of tasks (e.g. containers or virtual machines).
It is a table with worker names as keys. Values are worker structs.
The struct is represented via table. Keys are:
* [engine](#Engine) - Specify parameters of concrete engine (e.g. docker container);
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
A simple way to use "default" taskline. Same as:
```toml
[tasklines.""]
```


# Taskset
A table with names as keys and [tasks](#Task) as values.
Tasks in a taskset are supposed to run concurrently.
To provide order, a `requires` array could be used.
For example, the task `build` need to be run after the task setup:
```toml
[taskset.build]
requires = ["setup"]
# Other task parameters
```
Also tasks in a taskset could specify workers to run on using the
`workers` array, which consists of regexes of worker names.
By default, tasks run on all workers.


# Task
Tasks are defined by a [task type](#task-types) and have some parameters:
* `condition` - A shell command running on the worker.
    The task does not run if this fails;
* `parallel` - A bool controlling whether to run items tasks in parallel;
    [items](#Items) tasks should be executed in parallel;
* [vars](#task-vars) - Set variables;
* `clean-vars` - If true, run task without previously defined variables;
* `table` - Table.

## Task types
There are several types of tasks:
* [ensure](#ensure-task) - Ensure taskline could be run;
* [exec](#exec-task) - Run a command from an args array;
* [file](#file-task) - Copy a file to the worker;
* [get](#get-task) - Copy a file from the worker;
* [run-taskline](#runTaskline-task) - Run a taskline from the file;
* [run](#run-task) - Run a taskline;
* [shell](#shell-task) - Run a command from a shell string;
* [test](#test-task) - An array of commands.

## Ensure task
It has field `vars` with an array of variable names. Check them to be set.
Example of ensuring two variable `user` and `vars.lil` are set:
```toml
ensure.vars = ["user", "vars.lil"]
```

## Exec task
Consists of an `args` array of strings represented command and
[common command parameters](#common-command-parameters).

## File task
A file task has several fields:
* `dst` - Destination path on worker to store the file;
* `src` - Source path on host to get the file;
* `content` - String with contents of the file.
Example of creating `/tmp/test-file` on the worker:
```toml
file.dst = "/tmp/test-file"
file.content = "Test"
```

## Get task
A get task has several fields:
* `src` - Source path on worker to get the file;
* `dst` - Destination path on host to store the file. By default store file
    in the same directory as manifest located with a source file name.
Example of getting `/etc/os-release` from the worker:
```toml
get.src = "/etc/os-release"
```

## Run task
Run a taskline from manifest tasklines.

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

Example of a task installing `apt-repo` with `apt-get`:
```toml
run-taskline = { module = "apt-get", taskline = "install" }
vars.packages = [ "apt-repo" ]
```

## Shell task
Consists of a `command` string with a shell command and
[common command parameters](#common-command-parameters).

## Test task
Run commands. Fails on first failure command run with check. List of fields:
* `commands` - Contains an array of args or shell commands;
* `check` - Uses to overwrite default check value for commands.
Example of running two commands and printing theirs output:
```toml
test.commands = [
    { cmd = "echo lil", stdout = { print = true } },
    { cmd = "echo lal >&2", stderr = { print = true } },
]
```
Example of runnig several success of failure commands with check disabled by
default:
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

## Command output
Controls the redirection of the command output. Fields:
* `log` - Log output with a provided level;
* `print` - Print output to stdout.
For example, print stdout and log with `trace` level;
```toml
shell.command = "echo LiL"
shell.stdout = { print = true, log = "trace" }
```

## Matches
It is a formula consisting of `and`, `or` and `err-re`, `out-re`, `any-re`.
For example, `failure-matches` in a `shell` task;
```toml
shell.command = "ls LLM.toml"
shell.failure-matches = { or = [ { err-re = "LLM" }, { err-re = "toml" }]}
```

## Common command parameters
Some common command parameters:
* `check` - Fails the task if the command fails;
* `stdin` - Pass a provided string to the command's stdin;
* `stdout` - [Command output](#command-output) for stdout;
* `stderr` - [Command output](#command-output) for stderr;
* `success-codes` - Array of return codes treated as successful termination;
* `success-matches` - [Matches](#matches) that need to be matched for success;
* `failure-matches` - [Matches](#matches) that match means failure.

## Task vars
It is a table as in [vars](#Vars) or a list of tables as in
[extend vars](#extend-vars). Example of creating a shell task in a taskline
and setting a variable `target` and a `path` variable, which uses `target`:
```toml
[[taskline]]
shell.cmd = "ls -l {{ path }}"
vars = [
    { target = "debug" },
    { path = "target/{{ target }}/lineup" },
]
```


# Engine
Most engines have base fields:
* `name` - Set name for container or vm (default is worker's name);
* `setup` - Switch turning on or off setting engine up;
* `exists` - Set action performed then engine exists, variants are
    `fail`, `ignore` and `replace`.

There are several types of engines:
* [dbg](#dbg-engine);
* [docker](#docker-engine);
* [host](#host-engine);
* [podman](#podman-engine).
* [ssh](#ssh-engine);
* [vml](#vml-engine).


## Dbg engine
Engine used to debug tasks. Just print information about running tasks.
Could be setting any keys, all are ignored.

Example of debugging `vm` worker with `vml` keys:
```toml
#[workers.vm.engine.vml]
[workers.vm.engine.dbg]
image = "alt"
mem = "4G"
```


## Docker engine
Container engine using docker.
Docker specific options are:
* `memory` - Amount of memory;
* `image` - Image;
* `user` - User.

Example of a docker worker with name `docker` and `alt` image:
```toml
[workers.docker]
[workers.docker.engine.docker]
image = "alt"
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
Podman specific options are:
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
Container engine using podman.
Ssh specific options are:
* `host` - Ssh host;
* `port` - Ssh port;
* `user` - Ssh user;
* `key` - Ssh key;
* `ssh-cmd` - Ssh command (default to `["ssh"]`).

Example of a podman worker with name `localhost`:
```toml
[workers.localhost.engine.ssh]
host = "127.0.0.1"
```


## Vml engine
Virtual machine engine using vml.
Vml specific options are:
* `vml-bin` - Path to vml binary;
* `memory` - Amount of memory;
* `image` - Image;
* `net` - Describe network;
* `nproc` - Number of processes;
* `parent` - Create vm with specific parent;
* `user` - User.

Example of vml a worker with name `vm`, `alt` image and 2 gigabytes of memory:
```toml
[workers.vm]
[workers.vm.engine.vml]
image = "alt"
mem = "2G"
```


# Items
Items are used to multiply [workers](#worker-items) or [tasks](#task-items).
It sets a [template](templates.md) variable `item`, which can be used in
strings as `{{ item }}`. Items could be one of three forms:
1. Array of strings or integers
```toml
items = ["a", 2]
```
2. Sequence defined by `end`, `start` and `step`
```toml
items = { start = 1, end = 6, step = 2 }
```
3. Shell command, run on host, which stdout splitted by newlines
```toml
items = { command = "ls -d /lib*" }
```

## Worker items
Then used with workers, the item could be used in worker's name. Example of
creating to podman workers `buildbot-master` and `buildbot-worker`:
```toml
[workers."buildbot-{{item}}"]
items = ["master", "worker"]
[workers."buildbot-{{item}}".engine.podman]
image = "alt:sisyphus"
pod = "lineup-bb"
```

## Task items
In tasks items does not render in the task name. Instead, it creates several
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
* [vars](#extend-vars).

## Extend vars
Field `maps` gets a list of tables containing variables. Every element of the
list can use previous variables in templates. Example of setting a variable
`target` and a `path` variable, which uses `target`:
```toml
[extend]
vars.maps = [
    { target = "debug" },
    { path = "target/{{ target }}/lineup" },
]
```
