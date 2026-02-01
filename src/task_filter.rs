use std::collections::BTreeSet;

#[derive(Clone, Debug)]
struct Task {
    tasks: Vec<String>,
}

impl Task {
    fn empty() -> Self {
        Self { tasks: Vec::new() }
    }

    fn new<S: AsRef<str>>(tasks: S) -> Self {
        let tasks = tasks.as_ref().split('.').rev().map(String::from).collect();

        Self { tasks }
    }

    fn enter<S: AsRef<str>>(&mut self, task: S) {
        if self.tasks.pop_if(|t| t == task.as_ref()).is_none() {
            self.tasks = vec![];
        }
    }

    fn is_inner(&self) -> bool {
        self.tasks.len() == 1
    }

    fn task(&self) -> Option<&String> {
        self.tasks.last()
    }
}

#[derive(Clone, Debug)]
struct SetOfTasks {
    tasks: Vec<Task>,
    inner: BTreeSet<String>,
}

impl SetOfTasks {
    fn new(str_tasks: &[String]) -> Self {
        let mut tasks = Vec::new();
        let mut inner = BTreeSet::new();

        for task in str_tasks {
            let task = Task::new(task);
            if task.is_inner()
                && let Some(name) = task.task()
            {
                inner.insert(name.to_string());
            }
            tasks.push(task);
        }

        Self { tasks, inner }
    }

    fn enter<S: AsRef<str>>(&mut self, name: S) {
        let name = name.as_ref();
        let mut tasks = Vec::new();
        let mut inner = BTreeSet::new();

        for task in &self.tasks {
            let mut task = task.to_owned();
            task.enter(name);
            if task.is_inner()
                && let Some(name) = task.task()
            {
                inner.insert(name.to_string());
            }
            tasks.push(task);
        }

        self.tasks = tasks;
        self.inner = inner;
    }

    fn in_inner<S: AsRef<str>>(&self, name: S) -> bool {
        self.inner.contains(name.as_ref())
    }
}

#[derive(Clone, Debug)]
pub struct TaskFilter {
    taskset_first: Task,
    taskset_last: Task,
    taskset_skip: SetOfTasks,
}

impl TaskFilter {
    pub fn new() -> Self {
        Self {
            taskset_first: Task::empty(),
            taskset_last: Task::empty(),
            taskset_skip: SetOfTasks::new(&[]),
        }
    }

    pub fn taskset_enter<S: AsRef<str>>(&mut self, name: S) {
        let name = name.as_ref();
        self.taskset_first.enter(name);
        self.taskset_last.enter(name);
        self.taskset_skip.enter(name);
    }

    pub fn taskset_interval(&mut self, first: &Option<String>, last: &Option<String>) {
        self.taskset_first =
            if let Some(first) = first { Task::new(first) } else { Task::empty() };
        self.taskset_last = if let Some(last) = last { Task::new(last) } else { Task::empty() };
    }

    pub fn taskset_skip(&mut self, tasks: &[String]) {
        self.taskset_skip = SetOfTasks::new(tasks);
    }

    pub fn filter_layers(&self, layers: &Vec<Vec<String>>) -> Vec<Vec<String>> {
        let mut new_layers = vec![];
        let first = self.taskset_first.task();
        let last = self.taskset_last.task();
        let push = |layers: &mut Vec<Vec<String>>, mut layer: Vec<String>| {
            layer.retain(|t| !self.taskset_skip.in_inner(t));
            if !layer.is_empty() {
                layers.push(layer);
            }
        };

        let mut get: bool = false;
        if first.is_none() {
            get = true
        }
        for layer in layers {
            if let Some(first) = first
                && layer.contains(first)
            {
                if let Some(last) = last
                    && layer.contains(last)
                {
                    if first == last {
                        push(&mut new_layers, vec![first.to_string()]);
                    } else {
                        push(&mut new_layers, vec![first.to_string(), last.to_string()]);
                    }
                    break;
                } else {
                    push(&mut new_layers, vec![first.to_string()]);
                    get = true;
                    continue;
                }
            }

            if let Some(last) = last
                && layer.contains(last)
            {
                push(&mut new_layers, vec![last.to_string()]);
                break;
            }

            if get {
                push(&mut new_layers, layer.to_owned());
            }
        }

        new_layers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn empty_layers() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&None, &None);
        let layers = vec![];

        assert!(filter.filter_layers(&layers).is_empty());

        Ok(())
    }

    #[test]
    fn one_layer_just_first() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["first".to_string()]];

        assert_eq!(filter.filter_layers(&layers), layers);

        Ok(())
    }

    #[test]
    fn one_layer_other_first() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["first".to_string(), "other".to_string()]];

        assert_eq!(filter.filter_layers(&layers), vec![vec!["first".to_string()]]);

        Ok(())
    }

    #[test]
    fn one_layer_just_last() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["last".to_string()]];

        assert_eq!(filter.filter_layers(&layers), layers);

        Ok(())
    }

    #[test]
    fn one_layer_other_last() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["last".to_string(), "other".to_string()]];

        assert_eq!(filter.filter_layers(&layers), vec![vec!["last".to_string()]]);

        Ok(())
    }

    #[test]
    fn one_layer_just_both() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["first".to_string(), "last".to_string()]];

        assert_eq!(filter.filter_layers(&layers), layers);

        Ok(())
    }

    #[test]
    fn one_layer_other_both() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![vec!["first".to_string(), "last".to_string(), "other".to_string()]];

        assert_eq!(
            filter.filter_layers(&layers),
            vec![vec!["first".to_string(), "last".to_string()]]
        );

        Ok(())
    }

    #[test]
    fn one_layer_eq() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("first".to_string()));
        let layers = vec![vec!["first".to_string(), "last".to_string()]];

        assert_eq!(filter.filter_layers(&layers), vec![vec!["first".to_string()]]);

        Ok(())
    }

    #[test]
    fn last_first() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![
            vec!["ol".to_string(), "last".to_string()],
            vec!["first".to_string(), "of".to_string()],
        ];

        let result_layers = vec![vec!["last".to_string()]];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn multi() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("last".to_string()));
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["first".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["last".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn empty_first() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("".to_string()), &Some("last".to_string()));
        let layers = vec![
            vec!["pre".to_string()],
            vec!["".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![vec!["".to_string()], vec!["last".to_string()]];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn empty_last() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &Some("".to_string()));
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![vec!["first".to_string()], vec!["".to_string()]];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn multi_begin() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&None, &Some("last".to_string()));
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["last".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn multi_end() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first".to_string()), &None);
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["first".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn multi_dotted() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first.in".to_string()), &Some("last.out".to_string()));
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["first".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["last".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn skip_empty_layers() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_skip(&[]);
        let layers = vec![];

        assert!(filter.filter_layers(&layers).is_empty());

        Ok(())
    }

    #[test]
    fn skip_no_outer() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_skip(&["first.inner".to_string()]);
        let layers = vec![vec!["first".to_string()]];

        assert_eq!(filter.filter_layers(&layers), layers);

        Ok(())
    }

    #[test]
    fn skip_multi() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_skip(&["first".to_string(), "3".to_string(), "post".to_string()]);
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["pre".to_string()],
            vec!["of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["ol".to_string(), "last".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }

    #[test]
    fn enter_first() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("test.first".to_string()), &Some("final.last".to_string()));
        filter.taskset_enter("test");

        assert_eq!(filter.taskset_first.task(), Some(&"first".to_string()));
        assert!(filter.taskset_last.task().is_none());

        Ok(())
    }

    #[test]
    fn enter_last() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("setup.first".to_string()), &Some("test.last".to_string()));
        filter.taskset_enter("test");

        assert!(filter.taskset_first.task().is_none());
        assert_eq!(filter.taskset_last.task(), Some(&"last".to_string()));

        Ok(())
    }

    #[test]
    fn enter_both() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("test.first".to_string()), &Some("test.last".to_string()));
        filter.taskset_enter("test");

        assert_eq!(filter.taskset_first.task(), Some(&"first".to_string()));
        assert_eq!(filter.taskset_last.task(), Some(&"last".to_string()));

        Ok(())
    }

    #[test]
    fn enter_other() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("test.first".to_string()), &Some("test.last".to_string()));
        filter.taskset_enter("final");

        assert!(filter.taskset_first.task().is_none());
        assert!(filter.taskset_last.task().is_none());

        Ok(())
    }

    #[test]
    fn enter_twice() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(
            &Some("test.inner.first".to_string()),
            &Some("test.inner.last".to_string()),
        );
        filter.taskset_enter("test");
        filter.taskset_enter("inner");

        assert_eq!(filter.taskset_first.task(), Some(&"first".to_string()));
        assert_eq!(filter.taskset_last.task(), Some(&"last".to_string()));

        Ok(())
    }

    #[test]
    fn enter_skip() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_skip(&["test.inner.last".to_string()]);

        filter.taskset_enter("test");
        filter.taskset_enter("inner");
        assert!(filter.taskset_skip.in_inner("last"));

        Ok(())
    }

    #[test]
    fn inegral() -> Result<()> {
        let mut filter = TaskFilter::new();
        filter.taskset_interval(&Some("first.in".to_string()), &Some("last.out".to_string()));
        filter.taskset_skip(&["2".to_string(), "4".to_string(), "3.test".to_string()]);
        let layers = vec![
            vec!["pre".to_string()],
            vec!["first".to_string(), "of".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
            vec!["4".to_string(), "5".to_string()],
            vec!["ol".to_string(), "last".to_string()],
            vec!["post".to_string()],
        ];

        let result_layers = vec![
            vec!["first".to_string()],
            vec!["1".to_string()],
            vec!["3".to_string()],
            vec!["5".to_string()],
            vec!["last".to_string()],
        ];

        assert_eq!(filter.filter_layers(&layers), result_layers);

        Ok(())
    }
}
