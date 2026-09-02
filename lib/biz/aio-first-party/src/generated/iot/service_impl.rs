// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::IotService;
use anyhow::bail;

#[dill::component]
#[dill::interface(dyn IotService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct IotServiceImpl;

impl IotService for IotServiceImpl {
    fn post_devices(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("新建设备尚未实现")
        })
    }

    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("物联网状态尚未实现")
        })
    }

    fn post_templates_default_apply(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("初始化物联网模板尚未实现")
        })
    }

    fn post_devices_device_code_fixture_telemetry(
        &self,
        request: EndpointRequest,
    ) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("接收模拟遥测尚未实现")
        })
    }

    fn post_ui_action(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("物联网页面操作尚未实现")
        })
    }
}
