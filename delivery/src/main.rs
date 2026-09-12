mod build;
mod documents;

use std::{env, path::PathBuf, thread, time::Duration};

use anyhow::{Context as _, Result};
use az_plugin_delivery::{BuildJob, BuildReport};
use reqwest::blocking::Client;

pub struct Worker {
    client: Client,
    base: String,
    token: String,
    root: PathBuf,
}

impl Worker {
    pub fn request(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.token)
    }

    fn run(&self, job: &BuildJob) -> Result<()> {
        let root = self.root.join(format!("job-{}", job.id));
        let result = build::execute(self, job, &root);
        if let Err(error) = &result {
            if error.downcast_ref::<build::Retryable>().is_some()
                || error.chain().any(|cause| {
                    cause.downcast_ref::<reqwest::Error>().is_some_and(|e| {
                        e.is_timeout()
                            || e.is_connect()
                            || e.status().is_some_and(|status| status.is_server_error())
                    })
                })
            {
                return Err(anyhow::anyhow!("网络错误，保留产物等待租约重试: {error:#}"));
            }
        }
        let report = match result {
            Ok(documentation) => BuildReport {
                lease: job.lease.clone(),
                error: None,
                documentation,
            },
            Err(error) => BuildReport {
                lease: job.lease.clone(),
                error: Some(format!("{error:#}")),
                documentation: Default::default(),
            },
        };
        self.request(&format!("/api/internal/delivery/jobs/{}/complete", job.id))
            .json(&report)
            .send()?
            .error_for_status()?;
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let worker = Worker {
        client: Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()?,
        base: env::var("AIO_DELIVERY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3080".into())
            .trim_end_matches('/')
            .into(),
        token: env::var("AIO_DELIVERY_TOKEN").context("缺少 AIO_DELIVERY_TOKEN")?,
        root: env::var_os("AIO_DELIVERY_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| "/opt/aio-delivery".into()),
    };
    std::fs::create_dir_all(&worker.root)?;
    let mut delay = 10;
    loop {
        let result = (|| -> Result<()> {
            let job: Option<BuildJob> = worker
                .request("/api/internal/delivery/claim")
                .send()?
                .error_for_status()?
                .json()?;
            if let Some(job) = job {
                worker.run(&job)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("交付工作进程: {error:#}");
            delay = (delay * 2).min(300);
        } else {
            delay = 10;
        }
        thread::sleep(Duration::from_secs(delay));
    }
}
