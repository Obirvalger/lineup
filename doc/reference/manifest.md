# Manifest

Lineup manifest consists of several sections, most of which are optional.

* [vars](#Vars) - Set global variables;
* [workers](#Workers) - Define workers;
* [default](#Default) - Overwrite defaults;
* [tasklines](#Tasklines) - Define tasklines;
* [taskset](#Taskset) Define taskset.


# Vars
This is a table with string keys (variable names) and values of any type.
[Templates](templates.md) in strings are processed. For example, a bool
variable `build` stores the user's response to a question:
```toml
[vars]
build = "{{ confirm(msg='Do you want to build package?', default=true) }}"
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
* `vars` - Set variables as in the [vars](#Vars) section;
* `clean-vars` - If true, run task without previously defined variables;
* `table` - Table.

## Task types
There are several types of tasks:
* [command](#command-task) - Run a command from an args array;
* [shell](#shell-task) - Run a command from a shell string;
* [file](#file-task) - Copy a file to the worker;
* [run-taskline](#runTaskline-task) - Run a taskline from the file;
* [test](#test-task) - An array of commands.

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

## Command task
Consists of an `args` array of strings represented command and
[common command parameters](#common-command-parameters).

## Shell task
Consists of a `command` string with a shell command and
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

## RunTaskline task
Run a taskline from a file. Field:
* `taskline` - Name of the taskline (default is "");
* `file` - Path to the file containing the taskline
    (default is the current manifest);
* `module` - Name of a [module](modules.md) with the taskline;

Example of a task installing `apt-repo` with `apt-get`:
```toml
run-taskline = { module = "apt-get", taskline = "install" }
vars.packages = [ "apt-repo" ]
```

## Test task
Field `commands` with an array of args or shell commands.
Example of running two commands and printing theirs output:
```toml
test.commands = [
    { cmd = "echo lil", stdout = { print = true } },
    { cmd = "echo lal >&2", stderr = { print = true } },
]
```

# Engine
Most engines have base fields:
* `name` - Set name for container or vm (default is worker's name);
* `setup` - Switch turning on or off setting engine up;
* `exists` - Set action performed then engine exists, variants are
    `fail`, `ignore` and `replace`.

There are several types of engines:
* [host](#host-engine);
* [vml](#vml-engine);
* [docker](#docker-engine);
* [podman](#podman-engine).

## Host engine
Basic engine running commands just on your host.
Example of host worker with name `host`:
```toml
[workers.host]
engine = "host"
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

## Podman engine
Container engine using podman.
Docker specific options are:
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
