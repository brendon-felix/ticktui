use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

/// Priority levels matching the original TickTick encoding so existing
/// display / parsing code continues to work unchanged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum TaskPriority {
    #[default]
    None = 0,
    Low = 1,
    Medium = 3,
    High = 5,
}

impl TaskPriority {
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => Self::Low,
            3 => Self::Medium,
            5 => Self::High,
            _ => Self::None,
        }
    }

    pub fn to_i64(self) -> i64 {
        self as i64
    }
}

/// A task record as stored in / read from SQLite.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: i64,
    pub repeat_flag: String,
    pub status: i64,
    pub is_all_day: bool,
    pub sort_order: i64,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn priority(&self) -> TaskPriority {
        TaskPriority::from_i64(self.priority)
    }
}

// ---------------------------------------------------------------------------
// TaskData — the transfer object used by the UI and action system
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TaskData {
    pub title: Option<String>,
    pub task_id: Option<String>,
    pub project_id: Option<String>,
    pub content: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: Option<i64>,
    pub repeat_flag: Option<String>,
}

#[allow(dead_code)]
impl TaskData {
    pub fn from_task(task: &Task) -> Self {
        Self {
            title: Some(task.title.clone()),
            task_id: Some(task.id.clone()),
            project_id: Some(task.project_id.clone()),
            content: Some(task.content.clone()),
            due_date: task.due_date,
            priority: Some(task.priority),
            repeat_flag: if task.repeat_flag.is_empty() {
                None
            } else {
                Some(task.repeat_flag.clone())
            },
        }
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn task_id(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn project_id(mut self, project_id: String) -> Self {
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

    pub fn priority(mut self, priority: i64) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn repeat_flag(mut self, repeat_flag: String) -> Self {
        self.repeat_flag = Some(repeat_flag);
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

// ---------------------------------------------------------------------------
// TaskAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum TaskAction {
    Create,
    Edit,
    Complete,
    Delete,
}

// ---------------------------------------------------------------------------
// Date/time helpers
// ---------------------------------------------------------------------------

pub fn get_local_date() -> NaiveDate {
    Local::now().date_naive()
}

pub fn with_local_hms(date: NaiveDate, h: u32, m: u32, s: u32) -> DateTime<Utc> {
    let naive = date.and_time(NaiveTime::from_hms_opt(h, m, s).unwrap_or_default());
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_default()
        .with_timezone(&Utc)
}

pub fn is_overdue(now: DateTime<Utc>, task: &Task) -> bool {
    match task.due_date {
        Some(due) => due < now,
        None => false,
    }
}

pub fn is_due_today(now: DateTime<Utc>, task: &Task) -> bool {
    let today = now.with_timezone(&Local).date_naive();
    match task.due_date {
        Some(due) => due.with_timezone(&Local).date_naive() == today,
        None => false,
    }
}

pub fn is_due_tomorrow(now: DateTime<Utc>, task: &Task) -> bool {
    let tomorrow = now.with_timezone(&Local).date_naive() + Duration::days(1);
    match task.due_date {
        Some(due) => due.with_timezone(&Local).date_naive() == tomorrow,
        None => false,
    }
}

pub fn is_due_this_week(now: DateTime<Utc>, task: &Task) -> bool {
    let today = now.with_timezone(&Local).date_naive();
    let week_end = today + Duration::days(7);
    match task.due_date {
        Some(due) => {
            let d = due.with_timezone(&Local).date_naive();
            d >= today && d <= week_end
        }
        None => false,
    }
}

pub fn is_in_inbox(task: &Task) -> bool {
    task.project_id == "inbox"
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

pub fn sort_tasks(tasks: &mut Vec<Task>) {
    tasks.sort_by(|a, b| match (a.due_date, b.due_date) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(da), Some(db)) => da.cmp(&db),
    });
}

// ---------------------------------------------------------------------------
// RepeatFreq / RepeatFlag
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatFreq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Weekdays,
}

#[derive(Debug, Clone)]
pub struct RepeatFlag {
    freq: RepeatFreq,
    interval: u32,
    days: Option<Vec<Weekday>>,
}

impl RepeatFlag {
    pub fn new(freq: RepeatFreq, interval: u32, days: Option<Vec<Weekday>>) -> Self {
        Self {
            freq,
            interval,
            days,
        }
    }

    pub fn freq(&self) -> &RepeatFreq {
        &self.freq
    }

    pub fn days(&self) -> Option<&Vec<Weekday>> {
        self.days.as_ref()
    }

    pub fn from_string(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        let lower = s.to_lowercase();
        if lower.contains("rrule") {
            let freq = if lower.contains("daily") {
                RepeatFreq::Daily
            } else if lower.contains("weekly") {
                RepeatFreq::Weekly
            } else if lower.contains("monthly") {
                RepeatFreq::Monthly
            } else if lower.contains("yearly") || lower.contains("annual") {
                RepeatFreq::Yearly
            } else {
                return None;
            };
            let interval = extract_interval(&lower).unwrap_or(1);
            return Some(Self::new(freq, interval, None));
        }
        None
    }

    pub fn to_pretty_string(&self) -> String {
        let base = match self.freq {
            RepeatFreq::Daily => "Daily",
            RepeatFreq::Weekly => "Weekly",
            RepeatFreq::Monthly => "Monthly",
            RepeatFreq::Yearly => "Yearly",
            RepeatFreq::Weekdays => "Weekdays",
        };
        if self.interval > 1 {
            format!("Every {} {}s", self.interval, base.to_lowercase())
        } else {
            base.to_string()
        }
    }

    pub fn build(&self) -> String {
        let freq_str = match self.freq {
            RepeatFreq::Daily => "DAILY",
            RepeatFreq::Weekly => "WEEKLY",
            RepeatFreq::Monthly => "MONTHLY",
            RepeatFreq::Yearly => "YEARLY",
            RepeatFreq::Weekdays => "WEEKLY",
        };
        format!("RRULE:FREQ={};INTERVAL={}", freq_str, self.interval)
    }
}

fn extract_interval(s: &str) -> Option<u32> {
    s.split(';')
        .find(|part| part.starts_with("interval="))
        .and_then(|part| part.trim_start_matches("interval=").parse().ok())
}

pub fn format_repeat_flag(repeat_flag: &Option<String>) -> Option<String> {
    repeat_flag
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(RepeatFlag::from_string)
        .map(|rf| rf.to_pretty_string())
}
