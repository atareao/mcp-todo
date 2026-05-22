use chrono::{Datelike, Duration, Local, NaiveDate};

pub fn parse_natural_date(input: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let input = input.trim().to_lowercase();
    let now = Local::now();

    if let Some(dt) = parse_relative(&input, now) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    if let Some(dt) = parse_day_name(&input, now) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    if let Some(dt) = parse_date_string(&input) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    None
}

fn parse_relative(input: &str, now: chrono::DateTime<Local>) -> Option<chrono::DateTime<Local>> {
    let today = now.date_naive();

    match input {
        "today" => Some(now),
        "tomorrow" => Some((today + Duration::days(1)).and_hms_opt(9, 0, 0)?.and_local_timezone(Local).single()?),
        "yesterday" => Some((today - Duration::days(1)).and_hms_opt(9, 0, 0)?.and_local_timezone(Local).single()?),
        "next week" => Some((today + Duration::weeks(1)).and_hms_opt(9, 0, 0)?.and_local_timezone(Local).single()?),
        "next month" => {
            let next = today.month() + 1;
            let year = if next > 12 { today.year() + 1 } else { today.year() };
            let month = if next > 12 { next - 12 } else { next };
            NaiveDate::from_ymd_opt(year, month, 1)
                .and_then(|d| d.and_hms_opt(9, 0, 0))
                .map(|d| d.and_local_timezone(Local).single().unwrap())
        }
        "end of week" => {
            let days_until_sunday = 7 - today.weekday().num_days_from_monday();
            Some((today + Duration::days(days_until_sunday as i64)).and_hms_opt(18, 0, 0)?.and_local_timezone(Local).single()?)
        }
        "end of month" => {
            let (year, month) = (today.year(), today.month());
            let last_day = NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - Duration::days(1))
                .or_else(|| NaiveDate::from_ymd_opt(year, 12, 31));
            last_day.and_then(|d| d.and_hms_opt(18, 0, 0))
                .map(|d| d.and_local_timezone(Local).single().unwrap())
        }
        _ => parse_in_pattern(input, today),
    }
}

fn parse_in_pattern(input: &str, today: NaiveDate) -> Option<chrono::DateTime<Local>> {
    if input.starts_with("in ") {
        let rest = &input[3..];
        if let Some(days) = rest.strip_suffix(" days") {
            if let Ok(n) = days.trim().parse::<i64>() {
                return (today + Duration::days(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(day) = rest.strip_suffix(" day") {
            if let Ok(n) = day.trim().parse::<i64>() {
                return (today + Duration::days(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(weeks) = rest.strip_suffix(" weeks") {
            if let Ok(n) = weeks.trim().parse::<i64>() {
                return (today + Duration::weeks(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(week) = rest.strip_suffix(" week") {
            if let Ok(n) = week.trim().parse::<i64>() {
                return (today + Duration::weeks(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(months) = rest.strip_suffix(" months") {
            if let Ok(n) = months.trim().parse::<i32>() {
                let new_month = today.month() as i32 + n;
                let year = today.year() + (new_month - 1) / 12;
                let month = ((new_month - 1) % 12 + 1) as u32;
                return NaiveDate::from_ymd_opt(year, month, today.day())
                    .and_then(|d| d.and_hms_opt(9, 0, 0))
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(month) = rest.strip_suffix(" month") {
            if let Ok(n) = month.trim().parse::<i32>() {
                let new_month = today.month() as i32 + n;
                let year = today.year() + (new_month - 1) / 12;
                let month = ((new_month - 1) % 12 + 1) as u32;
                return NaiveDate::from_ymd_opt(year, month, today.day())
                    .and_then(|d| d.and_hms_opt(9, 0, 0))
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
    }

    if input.starts_with("last ") {
        let rest = &input[5..];
        if let Some(days) = rest.strip_suffix(" days") {
            if let Ok(n) = days.trim().parse::<i64>() {
                return (today - Duration::days(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
        if let Some(day) = rest.strip_suffix(" day") {
            if let Ok(n) = day.trim().parse::<i64>() {
                return (today - Duration::days(n)).and_hms_opt(9, 0, 0)
                    .map(|d| d.and_local_timezone(Local).single().unwrap());
            }
        }
    }

    None
}

fn parse_day_name(input: &str, now: chrono::DateTime<Local>) -> Option<chrono::DateTime<Local>> {
    let today = now.date_naive();
    let current_weekday = today.weekday();

    let (is_next, clean_input) = if input.starts_with("next ") {
        (true, &input[5..])
    } else {
        (false, input)
    };

    let target_weekday = match clean_input {
        "monday" | "lunes" => chrono::Weekday::Mon,
        "tuesday" | "martes" => chrono::Weekday::Tue,
        "wednesday" | "miercoles" | "miércoles" => chrono::Weekday::Wed,
        "thursday" | "jueves" => chrono::Weekday::Thu,
        "friday" | "viernes" => chrono::Weekday::Fri,
        "saturday" | "sabado" | "sábado" => chrono::Weekday::Sat,
        "sunday" | "domingo" => chrono::Weekday::Sun,
        _ => return None,
    };

    let days_ahead = if target_weekday.num_days_from_monday() > current_weekday.num_days_from_monday() {
        target_weekday.num_days_from_monday() - current_weekday.num_days_from_monday()
    } else {
        7 - (current_weekday.num_days_from_monday() - target_weekday.num_days_from_monday())
    };

    let days = if is_next { days_ahead + 7 } else { days_ahead };

    (today + Duration::days(days as i64))
        .and_hms_opt(9, 0, 0)
        .map(|d| d.and_local_timezone(Local).single().unwrap())
}

fn parse_date_string(input: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(input, fmt) {
            return date.and_hms_opt(9, 0, 0)
                .map(|d| d.and_utc());
        }
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(input) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    None
}

pub fn format_relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = dt - now;
    let days = duration.num_days();

    if days < 0 {
        let abs_days = (-days) as u64;
        match abs_days {
            0 => "today".to_string(),
            1 => "yesterday".to_string(),
            _ => format!("{} days ago", abs_days),
        }
    } else {
        let days = days as u64;
        match days {
            0 => "today".to_string(),
            1 => "tomorrow".to_string(),
            2..=6 => format!("in {} days", days),
            7..=13 => "next week".to_string(),
            _ => format!("in {} days", days),
        }
    }
}
