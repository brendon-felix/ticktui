use anyhow::Result;
use chrono::{DateTime, Local};
use ticks::{
    TickTick,
    projects::ProjectID,
    tasks::{Task, TaskID},
};

#[derive(Debug, Clone)]
pub enum TaskAction {
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

// pub async fn fetch_project_tasks(
//     client: &TickTick,
//     project_id: &ProjectID,
// ) -> Result<Vec<Task>, String> {
//     match client.get_project(project_id).await {
//         Ok(project) => match project.get_tasks().await {
//             Ok(tasks) => Ok(tasks),
//             Err(e) => Err(format!("Failed to fetch tasks from project: {:?}", e)),
//         },
//         Err(e) => Err(format!("Failed to get project: {:?}", e)),
//     }
// }

// pub async fn fetch_inbox_tasks(client: &TickTick) -> Result<Vec<Task>> {
//     let inbox_id = ProjectID("inbox".to_string());
//     client
//         .get_project_data(&inbox_id)
//         .await
//         .map_err(|e| anyhow::anyhow!("Failed to fetch inbox tasks: {:?}", e))
//         .map(|project_data| project_data.tasks)
// }

// pub async fn fetch_today_tasks(client: &TickTick) -> Result<Vec<Task>, String> {
//     let now = Local::now();
//     let today_end = now
//         .date_naive()
//         .and_hms_opt(23, 59, 59)
//         .unwrap()
//         .and_local_timezone(Local)
//         .unwrap()
//         .with_timezone(&chrono::Utc);

//     // Fetch all tasks from all projects
//     let all_tasks = match client.get_all_tasks_in_projects().await {
//         Ok(tasks) => tasks,
//         Err(e) => return Err(format!("Failed to fetch tasks: {:?}", e)),
//     };

//     // Fetch inbox tasks
//     let inbox_id = ProjectID("inbox".to_string());
//     let inbox_tasks = match client.get_project_data(&inbox_id).await {
//         Ok(project_data) => project_data.tasks,
//         Err(e) => return Err(format!("Failed to fetch inbox tasks: {:?}", e)),
//     };

//     let mut today_tasks = Vec::new();

//     // Filter tasks from all projects that are due today or overdue
//     for task in all_tasks {
//         let task_due = task.due_date;
//         // Check if due_date is set (not epoch) and is today or earlier (overdue)
//         if task_due.timestamp() > 0 && task_due <= today_end {
//             today_tasks.push(task);
//         }
//     }

//     // Filter inbox tasks that are due today or overdue
//     for task in inbox_tasks {
//         let task_due = task.due_date;
//         // Check if due_date is set (not epoch) and is today or earlier (overdue)
//         if task_due.timestamp() > 0 && task_due <= today_end {
//             today_tasks.push(task);
//         }
//     }

//     Ok(today_tasks)
// }

pub fn is_overdue(now: DateTime<Local>, task: &Task) -> bool {
    let now = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);
    task.due_date.timestamp() > 0 && task.due_date < now
}

// pub fn was_due_before_today(now: DateTime<Local>, task: &Task) -> bool {
//     let today_start = now
//         .date_naive()
//         .and_hms_opt(0, 0, 0)
//         .unwrap()
//         .and_local_timezone(Local)
//         .unwrap()
//         .with_timezone(&chrono::Utc);

//     task.due_date.timestamp() > 0 && task.due_date < today_start
// }

pub fn is_due_today(now: DateTime<Local>, task: &Task) -> bool {
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    let today_end = now
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    task.due_date >= today_start && task.due_date <= today_end
}

pub fn is_due_tomorrow(now: DateTime<Local>, task: &Task) -> bool {
    let tomorrow_start = (now + chrono::Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    let tomorrow_end = (now + chrono::Duration::days(1))
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    task.due_date.timestamp() > 0
        && task.due_date >= tomorrow_start
        && task.due_date <= tomorrow_end
}

pub fn is_due_this_week(now: DateTime<Local>, task: &Task) -> bool {
    let today_end = now
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    let week_end = (now + chrono::Duration::days(7))
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
        .with_timezone(&chrono::Utc);

    task.due_date.timestamp() > 0 && task.due_date >= today_end && task.due_date <= week_end
}

pub fn is_in_inbox(task: &Task) -> bool {
    task.project_id.0.starts_with("inbox")
    // true
}

// pub fn is_in_project(task: &Task, project_id: &ProjectID) -> bool {
//     &task.project_id.0 == &project_id.0
// }

// pub async fn create_task(
//     client: &TickTick,
//     title: String,
//     project: Option<ProjectID>,
//     content: Option<String>,
//     _description: Option<String>,
//     _priority: Option<TaskPriority>,
//     date: Option<NaiveDate>,
//     time: Option<NaiveTime>,
// ) -> Result<(), String> {
//     let mut builder = ticks::tasks::Task::builder(client, &title);
//     let project_id = project.unwrap_or(ProjectID("inbox".to_string()));
//     builder = builder.project_id(project_id);

//     if let Some(c) = content {
//         builder = builder.content(&c);
//     }

//     if let Some(d) = date {
//         let datetime = if let Some(t) = time {
//             d.and_time(t)
//         } else {
//             builder = builder.is_all_day(true);
//             d.and_hms_opt(0, 0, 0).unwrap()
//         };
//         let utc_datetime = chrono::Local
//             .from_local_datetime(&datetime)
//             .unwrap()
//             .to_utc();
//         builder = builder.due_date(utc_datetime);
//     }
//     match builder.build_and_publish().await {
//         Ok(_) => Ok(()),
//         Err(e) => Err(format!("Failed to create task: {:?}", e)),
//     }
// }

// pub async fn edit_task(
//     task: &mut Task,
//     title: Option<String>,
//     project: Option<ProjectID>,
//     content: Option<String>,
//     _description: Option<String>,
//     _priority: Option<TaskPriority>,
//     date: Option<NaiveDate>,
//     time: Option<NaiveTime>,
// ) -> Result<(), String> {
//     if let Some(t) = title {
//         task.title = t;
//     }
//     if let Some(p) = project {
//         task.project_id = p;
//     }
//     if let Some(c) = content {
//         task.content = c;
//     }
//     if let Some(d) = date {
//         let datetime = if let Some(t) = time {
//             // When time is provided, ensure the task is not all-day
//             task.is_all_day = false;
//             d.and_time(t)
//         } else {
//             task.is_all_day = true;
//             d.and_hms_opt(0, 0, 0).unwrap()
//         };
//         let utc_datetime = chrono::Local
//             .from_local_datetime(&datetime)
//             .unwrap()
//             .to_utc();
//         task.due_date = utc_datetime;
//         task.start_date = utc_datetime;
//     } else if time.is_some() {
//         // Handle case where only time is being updated without changing the date
//         if let Some(t) = time {
//             // If we have a valid due_date and we're setting a time, update to non-all-day
//             if task.due_date.timestamp() > 0 {
//                 task.is_all_day = false;
//                 let current_date = task.due_date.with_timezone(&chrono::Local).date_naive();
//                 let datetime = current_date.and_time(t);
//                 let utc_datetime = chrono::Local
//                     .from_local_datetime(&datetime)
//                     .unwrap()
//                     .to_utc();
//                 task.due_date = utc_datetime;
//                 task.start_date = utc_datetime;
//             }
//         }
//     }
//     task.publish_changes()
//         .await
//         .map_err(|e| format!("Failed to edit task: {:?}", e))
// }

pub async fn complete_task(
    client: &TickTick,
    project_id: &ProjectID,
    task_id: &TaskID,
) -> Result<(), String> {
    // Get a fresh task instance from the API with proper client context
    match client.get_project_data(project_id).await {
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

// pub async fn delete_task(task: Task) -> Result<(), String> {
//     match task.delete().await {
//         Ok(_) => Ok(()),
//         Err(e) => Err(format!("Failed to delete task: {:?}", e)),
//     }
// }

pub async fn delete_task(
    client: &TickTick,
    project_id: &ProjectID,
    task_id: &TaskID,
) -> Result<(), String> {
    // Get a fresh task instance from the API with proper client context
    match client.get_project_data(project_id).await {
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
