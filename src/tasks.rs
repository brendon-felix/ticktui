use anyhow::Result;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, Utc};
use ticks::{
    TickTick,
    projects::ProjectID,
    tasks::{Task, TaskID, TaskPriority},
};

#[derive(Debug, Clone, Default)]
pub struct TaskData {
    pub title: Option<String>,
    pub task_id: Option<TaskID>,
    pub project_id: Option<ProjectID>, // None means inbox
    // is_all_day: bool, // assumed true if time is 00:00:00
    // completed_time: Option<DateTime<Utc>>, // APi doesn't support completed tasks afaik
    pub content: Option<String>,
    // desc: Option<String>, // not really sure what this field does
    pub due_date: Option<DateTime<Utc>>,
    // subtasks: Vec<Subtask>, // not supported yet
    pub priority: Option<TaskPriority>,
    // reminders: Option<Vec<String>>, // not supported yet
    // repeat_flag: Option<String>, // not supported yet
    // sort_order, Option<i64>, // not supported
    // start_date: Option<DateTime<Utc>>, // not supported yet
    // status: Option<TaskStatus>, // again, not sure API even sends completed tasks
    // time_zone: Option<chrono::TimeZone>, // assume local timezone
    // tags: Vec<String>, // also don't think API supports this
}

#[allow(dead_code)]
impl TaskData {
    pub fn from_task(task: &Task) -> Self {
        Self {
            title: Some(task.title.clone()),
            task_id: Some(task.get_id().clone()),
            project_id: Some(task.project_id.clone()),
            content: Some(task.content.clone()),
            due_date: Some(task.due_date),
            priority: Some(task.priority),
        }
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn task_id(mut self, task_id: TaskID) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn project_id(mut self, project_id: ProjectID) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    pub fn due_date(mut self, due_date: DateTime<Utc>) -> Self {
        self.due_date = Some(due_date);
        self
    }

    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = Some(priority);
        self
    }
}

pub fn patch_task(task: &Task, data: TaskData) -> TaskData {
    let mut new_data = TaskData::from_task(task);

    if let Some(ref title) = data.title {
        new_data.title = Some(title.clone());
    }
    if let Some(ref content) = data.content {
        new_data.content = Some(content.clone());
    }
    if let Some(ref due_date) = data.due_date {
        new_data.due_date = Some(*due_date);
    }
    if let Some(ref priority) = data.priority {
        new_data.priority = Some(*priority);
    }

    new_data
}

#[derive(Debug, Clone)]
pub enum TaskAction {
    Create,
    Edit,
    Complete,
    Delete,
}

pub async fn fetch_all_tasks(client: &TickTick) -> Result<Vec<Task>> {
    let project_tasks = client
        .get_all_tasks_in_projects()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch tasks from projects: {:?}", e))?;
    let inbox_id = ProjectID("inbox".to_string());
    let inbox_tasks = client
        .get_project_data(&inbox_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch inbox tasks: {:?}", e))?
        .tasks;

    let mut all_tasks: Vec<Task> = project_tasks
        .into_iter()
        .chain(inbox_tasks.into_iter())
        .collect();

    sort_tasks(&mut all_tasks);

    Ok(all_tasks)
}

pub fn get_local_date(dt: DateTime<Utc>) -> NaiveDate {
    dt.with_timezone(&Local).date_naive()
}

pub fn with_local_hms(date: NaiveDate, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
    let local_dt = date
        .and_hms_opt(hour, min, sec)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    local_dt.with_timezone(&Utc)
}

pub fn is_overdue(now: DateTime<Utc>, task: &Task) -> bool {
    let today_start = with_local_hms(get_local_date(now), 0, 0, 0);
    if task.due_date.timestamp() > 0 && task.due_date < now {
        task.due_date != today_start
    } else {
        false
    }
}

pub fn is_due_today(now: DateTime<Utc>, task: &Task) -> bool {
    let today_start = with_local_hms(get_local_date(now), 0, 0, 0);
    let today_end = with_local_hms(get_local_date(now), 23, 59, 59);
    task.due_date >= today_start && task.due_date <= today_end
}

pub fn is_due_tomorrow(now: DateTime<Utc>, task: &Task) -> bool {
    let tomorrow = now + Duration::days(1);
    let tomorrow_start = with_local_hms(get_local_date(tomorrow), 0, 0, 0);
    let tomorrow_end = with_local_hms(get_local_date(tomorrow), 23, 59, 59);

    task.due_date.timestamp() > 0
        && task.due_date >= tomorrow_start
        && task.due_date <= tomorrow_end
}

pub fn is_due_this_week(now: DateTime<Utc>, task: &Task) -> bool {
    let today_end = with_local_hms(get_local_date(now), 23, 59, 59);
    let next_week = now + Duration::days(7);
    let week_end = with_local_hms(get_local_date(next_week), 23, 59, 59);

    task.due_date.timestamp() > 0 && task.due_date >= today_end && task.due_date <= week_end
}

pub fn is_in_inbox(task: &Task) -> bool {
    task.project_id.0.starts_with("inbox")
}

// pub fn is_in_project(task: &Task, project_id: &ProjectID) -> bool {
//     &task.project_id.0 == &project_id.0
// }

pub async fn create_task(client: &TickTick, data: TaskData) -> Result<(), String> {
    let mut builder = ticks::tasks::Task::builder(client, data.title.as_ref().unwrap());
    let project_id = data
        .project_id
        .clone()
        .unwrap_or(ProjectID("inbox".to_string()));
    builder = builder.project_id(project_id);

    if let Some(c) = data.content {
        builder = builder.content(&c);
    }

    if let Some(due_date) = data.due_date {
        builder = builder.due_date(due_date);
        // if time is 12:00 AM, set as all-day
        let time = due_date.with_timezone(&chrono::Local).time();
        if time == NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            builder = builder.is_all_day(true);
        }
    }

    if let Some(priority) = data.priority {
        builder = builder.priority(priority);
    }

    match builder.build_and_publish().await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to create task: {:?}", e)),
    }
}

pub async fn edit_task(client: &TickTick, data: TaskData) -> Result<(), String> {
    // Get a fresh task instance from the API with proper client context
    let project_id = data.project_id.clone().unwrap();
    let task_id = data.task_id.clone().unwrap();
    match client.get_project_data(&project_id).await {
        Ok(project_data) => {
            // Find the task in the project data
            if let Some(mut task) = project_data.tasks.into_iter().find(|t| {
                let t_id = t.get_id();
                t_id == &task_id
            }) {
                // Patch the task with new data
                let patched_data = patch_task(&task, data);

                if let Some(title) = patched_data.title {
                    task.title = title;
                }
                if let Some(content) = patched_data.content {
                    task.content = content;
                }
                if let Some(due_date) = patched_data.due_date {
                    task.due_date = due_date;
                }
                if let Some(priority) = patched_data.priority {
                    task.priority = priority;
                }

                match task.publish_changes().await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to edit task: {:?}", e)),
                }
            } else {
                Err("Task not found in project".to_string())
            }
        }
        Err(e) => Err(format!("Failed to get project data: {:?}", e)),
    }
}

pub async fn complete_task(client: &TickTick, data: TaskData) -> Result<(), String> {
    // Get a fresh task instance from the API with proper client context
    let project_id = data.project_id.unwrap();
    let task_id = data.task_id.unwrap();
    match client.get_project_data(&project_id).await {
        Ok(project_data) => {
            // Find the task in the project data
            let task_id_str = format!("{:?}", task_id);
            if let Some(mut task) = project_data.tasks.into_iter().find(|t| {
                let t_id_str = format!("{:?}", t.get_id());
                t_id_str == task_id_str
            }) {
                match task.complete().await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to complete task: {:?}", e)),
                }
            } else {
                Err("Task not found in project".to_string())
            }
        }
        Err(e) => Err(format!("Failed to get project data: {:?}", e)),
    }
}

pub async fn delete_task(client: &TickTick, data: TaskData) -> Result<(), String> {
    // Get a fresh task instance from the API with proper client context
    let project_id = data.project_id.unwrap();
    let task_id = data.task_id.unwrap();
    match client.get_project_data(&project_id).await {
        Ok(project_data) => {
            // Find the task in the project data
            let task_id_str = format!("{:?}", task_id);
            if let Some(task) = project_data.tasks.into_iter().find(|t| {
                let t_id_str = format!("{:?}", t.get_id());
                t_id_str == task_id_str
            }) {
                match task.delete().await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to delete task: {:?}", e)),
                }
            } else {
                Err("Task not found in project".to_string())
            }
        }
        Err(e) => Err(format!("Failed to get project data: {:?}", e)),
    }
}

pub fn sort_tasks(tasks: &mut Vec<Task>) {
    tasks.sort_by(|a, b| {
        use chrono::{DateTime, Datelike, Utc};

        // Helper to check if a datetime is the epoch (unset)
        let is_unset = |dt: &DateTime<Utc>| dt.timestamp() == 0;

        // Helper to compare dates by day only (year, month, day)
        let compare_by_day = |dt_a: &DateTime<Utc>, dt_b: &DateTime<Utc>| {
            match dt_a.year().cmp(&dt_b.year()) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }
            match dt_a.month().cmp(&dt_b.month()) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }
            dt_a.day().cmp(&dt_b.day())
        };

        // Compare due dates (unset dates go to the end)
        let due_cmp = match (is_unset(&a.due_date), is_unset(&b.due_date)) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => {
                // First compare by day
                let day_cmp = compare_by_day(&a.due_date, &b.due_date);
                if day_cmp != std::cmp::Ordering::Equal {
                    return day_cmp;
                }

                // Same day: prioritize non all-day tasks before all-day tasks
                match (a.is_all_day, b.is_all_day) {
                    (true, false) => return std::cmp::Ordering::Greater,
                    (false, true) => return std::cmp::Ordering::Less,
                    _ => {}
                }

                // Same day and same all-day status: compare by time
                a.due_date.cmp(&b.due_date)
            }
        };

        if due_cmp != std::cmp::Ordering::Equal {
            return due_cmp;
        }

        // If due dates are equal (including time), compare start dates (unset dates go to the end)
        let start_cmp = match (is_unset(&a.start_date), is_unset(&b.start_date)) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => {
                // First compare by day
                let day_cmp = compare_by_day(&a.start_date, &b.start_date);
                if day_cmp != std::cmp::Ordering::Equal {
                    return day_cmp;
                }

                // Same day: prioritize non all-day tasks before all-day tasks
                match (a.is_all_day, b.is_all_day) {
                    (true, false) => return std::cmp::Ordering::Greater,
                    (false, true) => return std::cmp::Ordering::Less,
                    _ => {}
                }

                // Same day and same all-day status: compare by time
                a.start_date.cmp(&b.start_date)
            }
        };

        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }

        // If all dates are equal, sort by sort_order
        a.sort_order.cmp(&b.sort_order)
    });
}
