use crate::audio::OutputFormat;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 创建下载任务时携带的通用选项。
///
/// 适用于单曲、专辑与多选批次共用的下载配置；这些选项会被固化进批次快照，供
/// 执行器、前端展示与后续重试逻辑共同使用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    /// 下载文件的根输出目录。
    pub output_dir: String,
    /// 目标输出格式。
    pub format: OutputFormat,
    /// 是否额外下载歌词侧车文件。
    pub download_lyrics: bool,
}

/// 下载任务的发起类型。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobKind {
    /// 下载单曲。
    Song,
    /// 下载整张专辑。
    Album,
    /// 下载来自多首歌曲选择集的批次。
    Selection,
}

/// 下载批次的聚合状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobStatus {
    /// 任务已创建，等待执行。
    Queued,
    /// 当前批次正在执行中。
    Running,
    /// 所有子任务均已成功完成。
    Completed,
    /// 至少有一项成功，且至少有一项失败或取消。
    PartiallyFailed,
    /// 没有成功项，且存在失败。
    Failed,
    /// 没有成功项，且被用户取消。
    Cancelled,
}

/// 单个下载子任务的执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadTaskStatus {
    /// 等待进入执行队列。
    Queued,
    /// 已被执行器取出，正在准备上下文。
    Preparing,
    /// 正在执行网络下载。
    Downloading,
    /// 正在执行磁盘写入、转码或标签写入。
    Writing,
    /// 当前子任务已成功完成。
    Completed,
    /// 当前子任务执行失败。
    Failed,
    /// 当前子任务被取消。
    Cancelled,
}

/// 下载错误的分类代码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadErrorCode {
    /// 网络传输或连接相关错误。
    Network,
    /// 上游 API 返回异常或数据不符合预期。
    Api,
    /// 本地文件系统读写错误。
    Io,
    /// 音频编解码或格式转换错误。
    Decode,
    /// 标签或元数据写入错误。
    Tagging,
    /// 歌词下载或写入错误。
    Lyrics,
    /// 用户主动取消。
    Cancelled,
    /// 请求参数不合法。
    InvalidRequest,
    /// 未归类的内部错误。
    Internal,
}

/// 面向前端暴露的下载错误信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadErrorInfo {
    /// 机器可读的错误分类。
    pub code: DownloadErrorCode,
    /// 适合直接展示或记录的错误摘要。
    pub message: String,
    /// 当前错误是否允许用户重试。
    pub retryable: bool,
    /// 可选的附加细节，用于调试或更细粒度提示。
    pub details: Option<String>,
}

/// 面向前端的单个下载子任务快照。
///
/// 适用于下载任务列表、任务详情面板与事件落地后的状态同步；返回值字段应被视为
/// 当前瞬时快照，而不是持续自动更新的引用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskSnapshot {
    /// 子任务 ID。
    pub id: String,
    /// 所属批次 ID。
    pub job_id: String,
    /// 歌曲 CID。
    pub song_cid: String,
    /// 歌曲名。
    pub song_name: String,
    /// 艺术家列表。
    pub artists: Vec<String>,
    /// 所属专辑 CID。
    pub album_cid: String,
    /// 所属专辑名称。
    pub album_name: String,
    /// 当前子任务状态。
    pub status: DownloadTaskStatus,
    /// 已完成字节数。
    pub bytes_done: u64,
    /// 总字节数；未知时为空。
    pub bytes_total: Option<u64>,
    /// 相对于根输出目录的归一化输出路径。
    pub output_path: Option<String>,
    /// 当前任务错误信息。
    pub error: Option<DownloadErrorInfo>,
    /// 当前重试次数。
    pub attempt: u32,
    /// 该歌曲在本批次中的下标，从 0 开始。
    pub song_index: usize,
    /// 本批次总歌曲数。
    pub song_count: usize,
}

/// `DownloadService` 内部使用的任务实体。
#[derive(Debug, Clone)]
pub struct InternalDownloadTask {
    pub id: String,
    pub job_id: String,
    pub song_cid: String,
    pub song_name: String,
    pub artists: Vec<String>,
    pub album_cid: String,
    pub album_name: String,
    pub status: DownloadTaskStatus,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub output_path: Option<String>,
    pub error: Option<DownloadErrorInfo>,
    pub attempt: u32,
    pub song_index: usize,
    pub song_count: usize,
    /// 当前任务使用的目标输出格式。
    pub format: crate::audio::OutputFormat,
    /// 是否为当前任务下载歌词侧车文件。
    pub download_lyrics: bool,
}

impl InternalDownloadTask {
    /// 将内部任务实体转换为供前端消费的快照结构。
    pub fn to_snapshot(&self, root_output_dir: &str) -> DownloadTaskSnapshot {
        DownloadTaskSnapshot {
            id: self.id.clone(),
            job_id: self.job_id.clone(),
            song_cid: self.song_cid.clone(),
            song_name: self.song_name.clone(),
            artists: self.artists.clone(),
            album_cid: self.album_cid.clone(),
            album_name: self.album_name.clone(),
            status: self.status,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            output_path: self
                .output_path
                .as_deref()
                .and_then(|path| normalize_snapshot_path(path, root_output_dir)),
            error: self.error.clone(),
            attempt: self.attempt,
            song_index: self.song_index,
            song_count: self.song_count,
        }
    }
}

/// `DownloadService` 内部使用的下载批次实体。
#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub id: String,
    pub kind: DownloadJobKind,
    pub status: DownloadJobStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub options: DownloadOptions,
    pub title: String,
    pub tasks: Vec<InternalDownloadTask>,
    pub error: Option<DownloadErrorInfo>,
}

impl DownloadJob {
    /// 将内部批次实体转换为供前端消费的快照结构。
    pub fn to_snapshot(&self) -> DownloadJobSnapshot {
        let completed_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Completed)
            .count();
        let failed_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Failed)
            .count();
        let cancelled_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Cancelled)
            .count();

        DownloadJobSnapshot {
            id: self.id.clone(),
            kind: self.kind,
            status: self.status,
            created_at: self.created_at.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            options: self.options.clone(),
            title: self.title.clone(),
            task_count: self.tasks.len(),
            completed_task_count,
            failed_task_count,
            cancelled_task_count,
            tasks: self
                .tasks
                .iter()
                .map(|t| t.to_snapshot(&self.options.output_dir))
                .collect(),
            error: self.error.clone(),
        }
    }

    /// 根据当前全部子任务状态推导批次状态。
    pub fn job_status(&self) -> DownloadJobStatus {
        if self.tasks.is_empty() {
            return self.status;
        }

        let has_non_terminal = self.tasks.iter().any(|task| {
            matches!(
                task.status,
                DownloadTaskStatus::Queued
                    | DownloadTaskStatus::Preparing
                    | DownloadTaskStatus::Downloading
                    | DownloadTaskStatus::Writing
            )
        });
        if has_non_terminal {
            return DownloadJobStatus::Running;
        }

        let completed_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Completed)
            .count();
        let failed_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Failed)
            .count();
        let cancelled_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Cancelled)
            .count();

        if completed_count == self.tasks.len() {
            return DownloadJobStatus::Completed;
        }

        if completed_count > 0 && (failed_count > 0 || cancelled_count > 0) {
            return DownloadJobStatus::PartiallyFailed;
        }

        if completed_count == 0 && failed_count > 0 {
            return DownloadJobStatus::Failed;
        }

        if completed_count == 0 && cancelled_count > 0 {
            return DownloadJobStatus::Cancelled;
        }

        self.status
    }
}

fn normalize_snapshot_path(path: &str, root_output_dir: &str) -> Option<String> {
    let path = Path::new(path);
    let root = Path::new(root_output_dir);

    path.strip_prefix(root)
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .ok()
        .filter(|relative| !relative.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        DownloadJob, DownloadJobKind, DownloadJobStatus, DownloadOptions, DownloadTaskStatus,
        InternalDownloadTask,
    };
    use crate::audio::OutputFormat;

    fn make_task(status: DownloadTaskStatus) -> InternalDownloadTask {
        InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status,
            bytes_done: 0,
            bytes_total: None,
            output_path: None,
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 2,
            format: OutputFormat::Flac,
            download_lyrics: true,
        }
    }

    fn make_job(tasks: Vec<InternalDownloadTask>) -> DownloadJob {
        DownloadJob {
            id: "job-1".to_string(),
            kind: DownloadJobKind::Album,
            status: DownloadJobStatus::Running,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            started_at: None,
            finished_at: None,
            options: DownloadOptions {
                output_dir: "/tmp".to_string(),
                format: OutputFormat::Flac,
                download_lyrics: true,
            },
            title: "Album".to_string(),
            tasks,
            error: None,
        }
    }

    #[test]
    fn returns_completed_when_all_tasks_completed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Completed),
            make_task(DownloadTaskStatus::Completed),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Completed);
    }

    #[test]
    fn returns_partially_failed_when_completed_and_failed_are_mixed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Completed),
            make_task(DownloadTaskStatus::Failed),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::PartiallyFailed);
    }

    #[test]
    fn returns_failed_when_no_task_completed_and_some_failed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Failed),
            make_task(DownloadTaskStatus::Cancelled),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Failed);
    }

    #[test]
    fn returns_cancelled_when_no_task_completed_and_only_cancelled() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Cancelled),
            make_task(DownloadTaskStatus::Cancelled),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Cancelled);
    }

    #[test]
    fn normalizes_output_path_to_relative_snapshot_form() {
        let task = InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status: DownloadTaskStatus::Completed,
            bytes_done: 0,
            bytes_total: None,
            output_path: Some("/Users/me/Downloads/SirenMusic/Album/Disc 1/Track.flac".to_string()),
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        };

        let snapshot = task.to_snapshot("/Users/me/Downloads/SirenMusic");

        assert_eq!(
            snapshot.output_path.as_deref(),
            Some("Album/Disc 1/Track.flac")
        );
    }

    #[test]
    fn omits_output_path_when_it_cannot_be_normalized_from_root() {
        let task = InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status: DownloadTaskStatus::Completed,
            bytes_done: 0,
            bytes_total: None,
            output_path: Some("/other-root/Album/Track.flac".to_string()),
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        };

        let snapshot = task.to_snapshot("/Users/me/Downloads/SirenMusic");

        assert_eq!(snapshot.output_path, None);
    }
}

/// 面向前端暴露的下载批次快照。
///
/// 适用于批次级列表、历史记录与详情展开视图；其中聚合计数字段与 `tasks` 内容
/// 始终基于同一次快照生成，调用方可把它们当作一致视图一起消费。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobSnapshot {
    /// 批次 ID。
    pub id: String,
    /// 下载发起类型。
    pub kind: DownloadJobKind,
    /// 当前批次状态。
    pub status: DownloadJobStatus,
    /// 创建时间。
    pub created_at: String,
    /// 开始执行时间。
    pub started_at: Option<String>,
    /// 结束时间。
    pub finished_at: Option<String>,
    /// 创建任务时使用的下载选项。
    pub options: DownloadOptions,
    /// 批次标题。
    pub title: String,
    /// 子任务总数。
    pub task_count: usize,
    /// 已完成子任务数。
    pub completed_task_count: usize,
    /// 失败子任务数。
    pub failed_task_count: usize,
    /// 已取消子任务数。
    pub cancelled_task_count: usize,
    /// 子任务快照列表。
    pub tasks: Vec<DownloadTaskSnapshot>,
    /// 批次级错误信息。
    pub error: Option<DownloadErrorInfo>,
}

/// 下载管理器的整体快照。
///
/// 适用于应用启动恢复、下载面板初始化或事件丢失后的全量兜底同步；返回值描述
/// 当前所有批次列表以及执行队列头部状态。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadManagerSnapshot {
    /// 当前所有批次。
    pub jobs: Vec<DownloadJobSnapshot>,
    /// 当前正在执行的批次 ID。
    pub active_job_id: Option<String>,
    /// 当前等待执行的批次 ID 列表。
    pub queued_job_ids: Vec<String>,
}

/// 前端创建下载批次时提交的请求载荷。
///
/// 适用于 command 层或测试直接构造批次创建请求；调用方应确保 `kind`、`song_cids`
/// 与 `album_cid` 的组合满足对应批次类型约束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDownloadJobRequest {
    /// 批次类型。
    pub kind: DownloadJobKind,
    /// 参与下载的歌曲 CID 列表。
    pub song_cids: Vec<String>,
    /// 专辑下载时使用的专辑 CID。
    pub album_cid: Option<String>,
    /// 通用下载选项。
    pub options: DownloadOptions,
}

/// 下载执行过程中发往前端的任务进度事件。
///
/// 适用于前端实时更新单个任务进度条、批次内当前位置与下载速度；该事件只描述
/// 当前一次上报瞬间的任务进度，不保证覆盖所有中间状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskProgressEvent {
    /// 所属批次 ID。
    pub job_id: String,
    /// 当前任务 ID。
    pub task_id: String,
    /// 当前任务状态。
    pub status: DownloadTaskStatus,
    /// 已完成字节数。
    pub bytes_done: u64,
    /// 总字节数；未知时为空。
    pub bytes_total: Option<u64>,
    /// 当前歌曲在本批次中的下标。
    pub song_index: usize,
    /// 本批次总歌曲数。
    pub song_count: usize,
    /// 最近采样窗口内估算出的下载速度，单位为字节每秒。
    pub speed_bytes_per_sec: f64,
}
