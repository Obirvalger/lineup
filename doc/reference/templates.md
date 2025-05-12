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
* [quote](#quote), `q` - Shell escapes value;
* [re_match](#re-match) - Regex match;
* [re_sub](#re-sub) - Regex sub.

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

## Re match
Matches a value to a regex from an argument `re`. Matches literally to a fixed
string in `re` if `fix` argument is true. If value is a string or number,
return a bool (true is matched). If value is an array, filter out from the
array all strings that does not match.

Example of showing warning if version does not match:
```toml
warn.msg = "Wrong version"
if = "! {{ '1.2.3' | re_match(re='1.2.2', fix=true) }}"
```

## Re sub
Substitute a value matched a regex from an argument `re` with a replacement
from a `str` argument.  Matches literally to a fixed string in `re` if `fix`
argument is true. Get substitution from a `str` argument (`$1` refers a first
group). If value is a string or number, return the value with a substitution.
If value is an array, substitute every element. If an argument `mathces_only`
is true, filter out from the array all strings that does not match.

Example of getting versions from an array:
```toml
info.msg = "{{ versions | re_sub(re='.*?(\\d(\\.\\d)*).*', str='$1', matches_only=true) }}"
vars.versions = ['ver-1.2.3', 'stable', '2.2-alt1', 3]
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
