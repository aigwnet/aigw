use std::time::Duration;

use crate::storage::tb_task::TbTask;
use chrono::Timelike;
use rbatis::{executor::RBatisTxExecutor, rbdc::DateTime};

pub(crate) struct Task {
    pub name: String,
    pub r#type: u32,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

pub async fn find_task(rb: &rbatis::RBatis, name: &str, r#type: u32) -> anyhow::Result<Task> {
    // 根据任务名称、类型读取任务，获取最后的任务时间，如果没有获取到记录，以分钟级聚合任务为例，最后的任务时间为当前时间前一分钟，并插入记录。
    if let Some(tb_task) = TbTask::select_by_name_and_type(rb, name, r#type).await? {
        return Ok(convert_tb_task(tb_task));
    }

    let now = DateTime::utc();
    let end_time = if r#type == 1 {
        get_one_mintue_ago()
    } else {
        get_one_hour_ago()
    };

    let tb_task = TbTask {
        id: None,
        name: Some(name.to_owned()),
        r#type: Some(r#type),
        last_time: Some(DateTime::from_timestamp(end_time.timestamp())),
        gmt_create: Some(now.clone()),
        gmt_modified: Some(now),
    };

    TbTask::insert(rb, &tb_task).await?;
    Ok(convert_tb_task(tb_task))
}

pub async fn update_task(rb: &RBatisTxExecutor, task: &Task) -> anyhow::Result<()> {
    let now = DateTime::utc();
    let tb_task = TbTask {
        id: None,
        name: None,
        r#type: None,
        last_time: Some(DateTime::from_timestamp(task.end_time.timestamp())),
        gmt_create: None,
        gmt_modified: Some(now),
    };
    let _ = TbTask::update_by_name_and_type(rb, &tb_task, &task.name, task.r#type).await?;
    Ok(())
}

fn convert_tb_task(tb_task: TbTask) -> Task {
    Task {
        name: tb_task.name.map_or("".to_string(), |s| s),
        r#type: tb_task.r#type.map_or(0, |i| i),
        end_time: tb_task.last_time.map_or(get_one_mintue_ago(), |t| {
            chrono::DateTime::from_timestamp(t.unix_timestamp(), 0)
                .map_or(get_one_mintue_ago(), |t| t)
        }),
    }
}

fn get_one_mintue_ago() -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now();
    let now = now.with_second(0).unwrap();
    let now = now.with_nanosecond(0).unwrap();
    now - Duration::from_secs(60)
}

fn get_one_hour_ago() -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now();
    let now = now.with_minute(0).unwrap();
    let now = now.with_second(0).unwrap();
    let now = now.with_nanosecond(0).unwrap();
    now - Duration::from_secs(3600)
}
