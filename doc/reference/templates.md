# Templates

Most of string literals support using templates in them.
[Tera](https://keats.github.io/tera/) template engine is used.


# Filters
There are lineup `filters` besides `tera` built-ins:
* `basename` - Trims all directories from value;
* [cond](#cond) - Adds one of two variants with respect to boolean value;
* `dirname` - Trims file name from value;
* [fs](#fs-filter) - Read `fs` variable;
* `is_empty` - Return true if array, object or string is empty;
* `json`, `j` - Encode value to json (alias to `json_encode`);
* `lines` - Split string value by newlines;
* [quote](#quote), `q` - Shell escapes value.

## Cond
Cond filter has two argument `if` and `else`. Return `if` argument if the value
is true and `else` argument otherwise. Default value for arguments is a `""`.
For example add a `--now` flag if `now` variable is true:
```toml
shell.command = "systemctl enable {{ now | cond(if='--now') }} docker"
```

## Fs filter
Read value stored in `fs` variable via kind. For example, save to variable
`var` value from `fs` varaiable `fs_var`:
```toml
var = "{{ 'fs_var' | fs }}"
```

## Quote
Quote filter works on scalar (bool, number and string) values and on arrays of
scalar values. It shell escapes strings, if value is an array, its elements
will be escaped and concatenated with value of `sep` argument inserted
between them. Default `sep` is a `" "`.
Example of removing packages stored in the `packages` variable:
```toml
shell.command = "apt-get remove -y {{ packages | quote }}"
```


# Functions
There are lineup `functions` besides `tera` built-ins:
* [confirm](#confirm) - Asks user a question and returns response as
    boolean value;
* [fs](#fs-function) - Read `fs` variable;
* [input](#input) - Prompt user for input;
* [host_cmd](#hostCmd) - Returns output from running on host command;
* `tmpdir` - Returns path to tmpdir.

## Confirm
Confirm function is used to get users response to a question. It has `msg`
argument with message showed to user. And `default` argument which presets
some value. Example of using confirm to set a boolean variable:
```toml
build = "{{ confirm(msg='Do you want to build package?', default=true) }}"
```

## Fs function
Read value stored in `fs` variable via kind. For example, save to variable
`var` value from `fs` varaiable `fs_var`:
```toml
var = "{{ fs(name='fs_var') }}"
```

## Input
Input function is used to get text from a user. It has a `msg` argument with
a message showed to the user. Example of getting task number or nothing:
```toml
vars.task = "{{ input(msg='Enter task number or just press <Enter>:') }}"
```

## HostCmd
Function host_cmd allows to capture an output of a running on the host
command.  Argument with command is named `cmd`. It could be a string or an
array of strings. In the case of a string, command runs as a shell command and
an array is passing to the exec. Boolean argument `check` determines returning
an error on the command failing then rendering the template. And a `capture`
argument controls what output stream should be captured - stdout or stderr, by
default stdout is captured. Example of capturing error message:
```toml
error_msg = "{{ host_cmd(cmd='ls /nothing', check=false, capture='stderr') }}"
```
