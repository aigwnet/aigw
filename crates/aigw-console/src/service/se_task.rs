use std::time::Duration;

use crate::storage::tb_task::TbTask;
use time::OffsetDateTime;

pub(crate) struct Task {
    pub name: String,
    pub r#type: u32,
    pub end_time: OffsetDateTime,
}

pub async fn find_task(rb: &sqlx::MySqlPool, name: &str, r#type: u32) -> anyhow::Result<Task> {
    // 根据任务名称、类型读取任务，获取最后的任务时间，如果没有获取到记录，以分钟级聚合任务为例，最后的任务时间为当前时间前一分钟，并插入记录。
    if let Some(tb_task) = TbTask::select_by_name_and_type(rb, name, r#type as i32).await? {
        return Ok(convert_tb_task(tb_task));
    }

    let now = OffsetDateTime::now_utc();
    let end_time = if r#type == 1 {
        get_one_mintue_ago()
    } else {
        get_one_hour_ago()
    };

    let tb_task = TbTask {
        id: None,
        name: Some(name.to_owned()),
        r#type: Some(r#type as i32),
        last_time: Some(
            OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        ),
        gmt_create: Some(now),
        gmt_modified: Some(now),
    };

    TbTask::insert(rb, &tb_task).await?;
    Ok(convert_tb_task(tb_task))
}

pub async fn update_task<'e, E: sqlx::MySqlExecutor<'e>>(
    rb: E,
    task: &Task,
) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    let tb_task = TbTask {
        id: None,
        name: None,
        r#type: None,
        last_time: Some(
            OffsetDateTime::from_unix_timestamp(task.end_time.unix_timestamp())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        ),
        gmt_create: None,
        gmt_modified: Some(now),
    };
    let _ = TbTask::update_by_name_and_type(rb, &tb_task, &task.name, task.r#type as i32).await?;
    Ok(())
}

fn convert_tb_task(tb_task: TbTask) -> Task {
    Task {
        name: tb_task.name.unwrap_or("".to_string()),
        r#type: tb_task.r#type.map_or(0, |i| i as u32),
        end_time: tb_task.last_time.map_or(get_one_mintue_ago(), |t| {
            OffsetDateTime::from_unix_timestamp(t.unix_timestamp())
                .unwrap_or(get_one_mintue_ago())
        }),
    }
}

fn get_one_mintue_ago() -> OffsetDateTime {
    OffsetDateTime::now_utc() - Duration::from_secs(60)
}

fn get_one_hour_ago() -> OffsetDateTime {
    OffsetDateTime::now_utc() - Duration::from_secs(3600)
}
