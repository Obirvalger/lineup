# Manifest

Lineup manifest consists of several sections. Most of all are optional.

* [vars](#Vars) - Set global variables;
* [workers](#Workers) - Define workers;
* [default](#Default) - Overwrite defaults;
* [tasklines](#Tasklines) - Define tasklines;
* [taskset](#Taskset) Define taskset.


# Vars
It is a table with string keys (variable names) and values of any type.
[Templates](templates.md) in strings are processed. For example bool variable
`build` stores users response to a question:
```toml
[vars]
build = "{{ confirm(msg='Do you want to build package?', default=true) }}"
```


# Workers
Workers describe runners of tasks (e.g. containers or virtual machines).
It is a table with worker names as keys. Values is a worker struct.
The struct is represented via table. Keys are:
* [engine](#Engine) - Specify parameters of concrete engine (e.g. docker container);
* [items](#Items) - Multiplier to create several workers;
* `table-by-item` - Table indexed by item value;
* `table-by-name` - Table indexed by name.


# Default
Set defaults. For now it has only worker defaults.
For example use alt podman container:
```toml
[default]
[default.worker.engine.podman]
image = "alt"
```


# Tasklines
Array of [tasks](#Task) that are run sequentially.


# Taskline
Simple way to use "default" taskline. Same as:
```toml
[tasklines.""]
```


# Taskset
Table with names as keys and [tasks](#Task) as values.
Tasks in taskset suppose to run concurrently.
To provide order `requires` array could be used.
For example task `build` need to be run after task setup:
```toml
[taskset.build]
requires = ["setup"]
# Other task parameters
```
Also tasks in taskset could specify workers to run on using `workers` array.
That array consist of regexes of worker names.
By default tasks run on all workers.


# Task
Tasks are defined by [task type](#task-types) and have some parameters:
* `condition` - Shell command running on worker. Does not run task if failed;
* `parallel` - Bool, controlling run [items](#Items) tasks parallel;
* `vars` - Set variables as in [vars](#Vars) section;
* `clean-vars` - If true, run task without upper set variables;
* `table` - Table.

## Task types
There are several types of tasks:
* [command](#command-task) - Run command from args array;
* [shell](#shell-task) - Run command from shell string;
* [file](#file-task) - Copy file to worker;
* [run-taskline](#runTaskline-task) - Run taskline from file;
* [test](#test-task) - Array of args or shell commands.

## Command output
Controls output. Field:
* `log` - Log output with provided level;
* `print` - Print output to stdout.
For example print stdout and log with `trace` level;
```toml
shell.command = "echo LiL"
shell.stdout = { print = true, log = "trace" }
```

## Matches
It is a formula consists of `and`, `or` and `err-re`, `out-re`, `any-re`.
For example `failure-matches` in `shell` task;
```toml
shell.command = "ls LLM.toml"
shell.failure-matches = { or = [ { err-re = "LLM" }, { err-re = "toml" }]}
```

## Common command parameters
There are some common command parameters:
* `check` - Fails task if command fails;
* `stdin` - Pass provided string to commands stdin;
* `stdout` - [Command output](#command-output) for stdout;
* `stderr` - [Command output](#command-output) for stderr;
* `success-codes` - Array of return codes treated as successful termination;
* `success-matches` - [Matches](#matches) that need to be matched for success;
* `failure-matches` - [Matches](#matches) that match means failure.

## Command task
Consist of `args` array of strings represented command and
[common command parameters](#common-command-parameters).

## Shell task
Consist of `command` string with shell command and
[common command parameters](#common-command-parameters).

## File task
File task has several fields:
* `dst` - Destination path on worker to store file;
* `src` - Source path on host to get file;
* `content` - String with contents of file.
Example of creating `/tmp/test-file` on worker:
```toml
file.dst = "/tmp/test-file"
file.content = "Test"
```

## RunTaskline task
Run taskline for current or another file. Field:
* `taskline` - Name of taskline (default is "");
* `file` - Path to file containing taskline (default is current manifest);
* `module` - Name of [module](modules.md) with taskline;

Example of task installing `apt-repo` with `apt-get`:
```toml
run-taskline = { module = "apt-get", taskline = "install" }
vars.packages = [ "apt-repo" ]
```

## Test task
Field `commands` with array of args or shell commands.
Example of running two commands and printing theirs output:
```toml
test.commands = [
    { cmd = "echo lil", stdout = { print = true } },
    { cmd = "echo lal >&2", stderr = { print = true } },
]
```

# Engine
Describe concrete engine. Most of all engines have base fields:
* `name` - Set name for container or vm (default in workers name);
* `setup` - Switch turning on or of setting engine up;
* `exists` - Set action performed then engine exists, variants are
    `fail`, `ignore` and `replace`.

There are several types of engines:
* [host](#host-engine);
* [vml](#vml-engine);
* [docker](#docker-engine);
* [podman](#podman-engine).

## Host engine
Basic engine running command just on your host.
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

Example of vml worker with name `vm`, `alt` image and 2 gigabytes of memory:
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

Example of docker worker with name `docker` and `alt` image:
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

Example of podman worker with name `podman` and `alt` image:
```toml
[workers.podman]
[workers.podman.engine.podman]
image = "alt"
```


# Items
Items used to multiply [workers](#worker-items) or [tasks](#task-items).
It sets [template](templates.md) variable `item`, which could be used in
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
Then used with workers, item could be used in workers name. Example of
creating to podman workers `buildbot-master` and `buildbot-worker`:
```toml
[workers."buildbot-{{item}}"]
items = ["master", "worker"]
[workers."buildbot-{{item}}".engine.podman]
image = "alt:sisyphus"
pod = "lineup-bb"
```

## Task items
In tasks items does not change task name. Instead it creates several tasks with
the same name. It is a sort of loop task. By default tasks run in parallel.
```toml
shell.cmd = "echo {{ item }}"
shell.stdout = { print = true }
items = { start = 1, end = 6, step = 2 }
parallel = false
```
