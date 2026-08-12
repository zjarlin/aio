use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result, anyhow, bail};
use az_admin_shell_core::{
    AdminCommand, AdminCompiler, AdminDefinition, DefinitionId, ExtensionType, MenuDefinition,
    PageDefinition, PageExtensionCompilerIndex, PageRendererDefinition, ResourceCatalog,
    ResourceDefinition, ResourceFieldDefinition, ResourceFieldKind, ResourceOperations,
    ResourcePage, ResourceRecord, ResourceRequest, ResourceResponse, SceneDefinition,
};
use az_dioxus_admin_extension_crud::{CrudPageConfig, CrudPageExtension};
use az_dioxus_admin_shell::{
    AdminFuture, AdminProvider, AdminSnapshot, ConventionFileResult, DynAdminProvider,
};
use serde_json::{Map, Value};
use studio::{
    ChildCollection, DefinitionState, DraftSnapshot, EditableProperty, GraphEntity, GraphPatch,
    GraphPatchBatch, MenuRowActions, PatchOrigin, RuntimeRecordInput, RuntimeRecordPage,
    RuntimeRecordView, SymbolId, TableDefinition,
    browser_http::{delete_api, get_api, patch_api, post_api},
};

use super::runtime_extension::{
    AioRuntimePageConfig, AioRuntimePageExtension, AioStudioPageExtension,
};

const STUDIO_SCENE_ID: &str = "00000000-0000-4000-8000-000000000001";
const STUDIO_MENU_ID: &str = "00000000-0000-4000-8000-000000000002";
const STUDIO_PAGE_ID: &str = "00000000-0000-4000-8000-000000000003";

#[derive(Clone, Debug, Default)]
struct AioAdminProvider;

impl AdminProvider for AioAdminProvider {
    fn key(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn load(&self) -> AdminFuture<AdminSnapshot> {
        Box::pin(async { load_snapshot().await })
    }

    fn execute(&self, command: AdminCommand) -> AdminFuture<AdminSnapshot> {
        Box::pin(async move {
            let draft = get_api::<DraftSnapshot>("", "/api/studio/program/draft")
                .await
                .map_err(|error| anyhow!(error))?;
            let patches = command_patches(&draft, command)?;
            let batch = GraphPatchBatch {
                base_version: draft.version,
                patches,
                origin: PatchOrigin::Studio,
            };
            let draft = patch_api::<_, DraftSnapshot>("", "/api/studio/program/draft", &batch)
                .await
                .map_err(|error| anyhow!(error))?;
            snapshot_from_draft(draft)
        })
    }

    fn execute_resource(&self, request: ResourceRequest) -> AdminFuture<ResourceResponse> {
        Box::pin(async move { execute_resource(request).await })
    }

    fn generate_convention_file(&self, page_id: DefinitionId) -> AdminFuture<ConventionFileResult> {
        Box::pin(async move {
            let page_id = symbol_id(&page_id)?;
            let path = format!("/api/studio/program/pages/{page_id}/convention-file");
            let result = post_api::<_, studio::ConventionFileResult>("", &path, &Value::Null)
                .await
                .map_err(|error| anyhow!(error))?;
            Ok(ConventionFileResult {
                path: result.path,
                created: result.created,
            })
        })
    }
}

#[rudi::Singleton(name = std::any::type_name::<AioAdminProvider>())]
fn admin_provider() -> DynAdminProvider {
    Arc::new(AioAdminProvider)
}

async fn load_snapshot() -> Result<AdminSnapshot> {
    let draft = get_api::<DraftSnapshot>("", "/api/studio/program/draft")
        .await
        .map_err(|error| anyhow!(error))?;
    snapshot_from_draft(draft)
}

fn snapshot_from_draft(draft: DraftSnapshot) -> Result<AdminSnapshot> {
    let resources = resource_catalog(&draft.definition);
    let definition = admin_definition(&draft.definition)?;
    let mut context = rudi::Context::auto_register();
    let extensions = PageExtensionCompilerIndex::from_context(&mut context)?;
    let compiled = AdminCompiler::new(&extensions, &resources)
        .compile(&definition)
        .map_err(|diagnostics| anyhow!("编译 AIO 工作台失败: {diagnostics:?}"))?;
    Ok(AdminSnapshot {
        definition,
        compiled,
        resources,
    })
}

fn admin_definition(program: &studio::ProgramDefinition) -> Result<AdminDefinition> {
    let mut scenes = program
        .menus
        .iter()
        .map(|scene| {
            Ok(SceneDefinition {
                id: definition_id(scene.id),
                name: scene.name.clone(),
                title: scene.title.clone(),
                menus: scene
                    .children
                    .iter()
                    .map(admin_menu)
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let studio_page_id = DefinitionId::from_value(STUDIO_PAGE_ID);
    let studio_menu = MenuDefinition {
        id: DefinitionId::from_value(STUDIO_MENU_ID),
        name: "studio".to_owned(),
        title: "Studio".to_owned(),
        icon: None,
        page_id: Some(studio_page_id.clone()),
        enabled: true,
        children: Vec::new(),
    };
    if let Some(scene) = scenes.first_mut() {
        scene.menus.insert(0, studio_menu);
    } else {
        scenes.push(SceneDefinition {
            id: DefinitionId::from_value(STUDIO_SCENE_ID),
            name: "studio".to_owned(),
            title: "Studio".to_owned(),
            menus: vec![studio_menu],
        });
    }
    let mut pages = program
        .pages
        .iter()
        .map(|page| {
            Ok(PageDefinition {
                id: definition_id(page.id),
                name: page.name.clone(),
                title: page.title.clone(),
                renderer: admin_renderer(&page.renderer)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    pages.insert(
        0,
        PageDefinition {
            id: studio_page_id,
            name: "studio".to_owned(),
            title: "Studio".to_owned(),
            renderer: PageRendererDefinition::Extension {
                extension_type: ExtensionType::of::<AioStudioPageExtension>(),
                schema_version: 1,
                config: Value::Object(Default::default()),
            },
        },
    );
    Ok(AdminDefinition {
        id: definition_id(program.id),
        name: program.name.clone(),
        title: program.title.clone(),
        scenes,
        pages,
    })
}

fn admin_menu(menu: &studio::MenuDefinition) -> Result<MenuDefinition> {
    Ok(MenuDefinition {
        id: definition_id(menu.id),
        name: menu.name.clone(),
        title: menu.title.clone(),
        icon: menu.icon.clone(),
        page_id: menu.page_id.map(definition_id),
        enabled: menu.enabled,
        children: menu
            .children
            .iter()
            .map(admin_menu)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn admin_renderer(renderer: &studio::PageRendererDefinition) -> Result<PageRendererDefinition> {
    match renderer {
        studio::PageRendererDefinition::ConventionFile => {
            Ok(PageRendererDefinition::ConventionFile)
        }
        studio::PageRendererDefinition::Extension {
            extension_type,
            schema_version,
            config,
        } => Ok(PageRendererDefinition::Extension {
            extension_type: ExtensionType::from_provider_key(extension_type.clone()),
            schema_version: *schema_version,
            config: config.clone(),
        }),
        studio::PageRendererDefinition::CrudTable { table } => {
            let config = CrudPageConfig {
                resource_id: table.model_id.map_or_else(String::new, |id| id.to_string()),
                page_size: table.page_size,
            };
            Ok(PageRendererDefinition::Extension {
                extension_type: ExtensionType::of::<CrudPageExtension>(),
                schema_version: 1,
                config: serde_json::to_value(config).context("序列化 CRUD 页面配置失败")?,
            })
        }
        studio::PageRendererDefinition::MenuTree
        | studio::PageRendererDefinition::TreeTable { .. } => {
            Ok(PageRendererDefinition::Extension {
                extension_type: ExtensionType::of::<AioRuntimePageExtension>(),
                schema_version: 1,
                config: serde_json::to_value(AioRuntimePageConfig {
                    renderer: renderer.clone(),
                })
                .context("序列化 AIO 运行时页面配置失败")?,
            })
        }
    }
}

fn command_patches(draft: &DraftSnapshot, command: AdminCommand) -> Result<Vec<GraphPatch>> {
    let definition = &draft.definition;
    match command {
        AdminCommand::SetApplicationTitle { title } => Ok(vec![GraphPatch::SetProperty {
            target_id: definition.id,
            property: EditableProperty::Title,
            value: Value::String(title),
        }]),
        AdminCommand::AddScene { scene } => Ok(vec![GraphPatch::Insert {
            parent_id: definition.id,
            collection: ChildCollection::Menus,
            index: definition.menus.len(),
            entity: GraphEntity::Menu(studio_scene(scene)?),
        }]),
        AdminCommand::AddMenu {
            scene_id,
            parent_menu_id,
            menu,
        } => {
            let parent_id = parent_menu_id.as_ref().unwrap_or(&scene_id);
            Ok(vec![GraphPatch::Insert {
                parent_id: symbol_id(parent_id)?,
                collection: ChildCollection::MenuChildren,
                index: menu_child_count(&definition.menus, symbol_id(parent_id)?)?,
                entity: GraphEntity::Menu(studio_menu(menu)?),
            }])
        }
        AdminCommand::AddPage { page } => Ok(vec![GraphPatch::Insert {
            parent_id: definition.id,
            collection: ChildCollection::Pages,
            index: definition.pages.len(),
            entity: GraphEntity::Page(studio_page(page)?),
        }]),
        AdminCommand::AddMenuPage {
            scene_id,
            parent_menu_id,
            menu,
            page,
        } => {
            let parent_id = parent_menu_id.as_ref().unwrap_or(&scene_id);
            let old_page = studio_page(page)?;
            let route = studio::RouteDefinition {
                id: SymbolId::new(),
                name: old_page.name.clone(),
                path: unique_route_path(definition, &old_page.name),
                page_id: old_page.id,
                state: DefinitionState::Known,
                required_permissions: Vec::new(),
            };
            Ok(vec![
                GraphPatch::Insert {
                    parent_id: definition.id,
                    collection: ChildCollection::Pages,
                    index: definition.pages.len(),
                    entity: GraphEntity::Page(old_page),
                },
                GraphPatch::Insert {
                    parent_id: definition.id,
                    collection: ChildCollection::Routes,
                    index: definition.routes.len(),
                    entity: GraphEntity::Route(route),
                },
                GraphPatch::Insert {
                    parent_id: symbol_id(parent_id)?,
                    collection: ChildCollection::MenuChildren,
                    index: menu_child_count(&definition.menus, symbol_id(parent_id)?)?,
                    entity: GraphEntity::Menu(studio_menu(menu)?),
                },
            ])
        }
        AdminCommand::SetPageRenderer { page_id, renderer } => Ok(vec![GraphPatch::SetProperty {
            target_id: symbol_id(&page_id)?,
            property: EditableProperty::PageRenderer,
            value: serde_json::to_value(studio_renderer(renderer)?)
                .context("序列化 AIO 页面渲染配置失败")?,
        }]),
        AdminCommand::DeleteScene { scene_id } => Ok(vec![GraphPatch::Delete {
            target_id: symbol_id(&scene_id)?,
        }]),
        AdminCommand::DeleteMenu { menu_id, .. } => Ok(vec![GraphPatch::Delete {
            target_id: symbol_id(&menu_id)?,
        }]),
        AdminCommand::DeletePage { page_id } => {
            let page_id = symbol_id(&page_id)?;
            let mut patches = definition
                .routes
                .iter()
                .filter(|route| route.page_id == page_id)
                .map(|route| GraphPatch::Delete {
                    target_id: route.id,
                })
                .collect::<Vec<_>>();
            patches.push(GraphPatch::Delete { target_id: page_id });
            Ok(patches)
        }
    }
}

fn studio_scene(scene: SceneDefinition) -> Result<studio::MenuDefinition> {
    Ok(studio::MenuDefinition {
        id: symbol_id(&scene.id)?,
        name: scene.name,
        title: scene.title,
        state: DefinitionState::Known,
        icon: None,
        page_id: None,
        enabled: true,
        children: scene
            .menus
            .into_iter()
            .map(studio_menu)
            .collect::<Result<Vec<_>>>()?,
        required_permissions: Vec::new(),
        row_actions: MenuRowActions::default(),
    })
}

fn studio_menu(menu: MenuDefinition) -> Result<studio::MenuDefinition> {
    Ok(studio::MenuDefinition {
        id: symbol_id(&menu.id)?,
        name: menu.name,
        title: menu.title,
        state: DefinitionState::Known,
        icon: menu.icon,
        page_id: menu.page_id.as_ref().map(symbol_id).transpose()?,
        enabled: menu.enabled,
        children: menu
            .children
            .into_iter()
            .map(studio_menu)
            .collect::<Result<Vec<_>>>()?,
        required_permissions: Vec::new(),
        row_actions: MenuRowActions::default(),
    })
}

fn studio_page(page: PageDefinition) -> Result<studio::PageDefinition> {
    Ok(studio::PageDefinition {
        id: symbol_id(&page.id)?,
        name: page.name,
        title: page.title,
        state: DefinitionState::Known,
        renderer: studio_renderer(page.renderer)?,
        endpoints: Vec::new(),
    })
}

fn studio_renderer(renderer: PageRendererDefinition) -> Result<studio::PageRendererDefinition> {
    match renderer {
        PageRendererDefinition::ConventionFile => {
            Ok(studio::PageRendererDefinition::ConventionFile)
        }
        PageRendererDefinition::Extension {
            extension_type,
            config,
            ..
        } if extension_type == ExtensionType::of::<CrudPageExtension>() => {
            let config = serde_json::from_value::<CrudPageConfig>(config)
                .context("解析 CRUD 页面配置失败")?;
            let model_id = if config.resource_id.trim().is_empty() {
                None
            } else {
                Some(SymbolId::parse(&config.resource_id)?)
            };
            Ok(studio::PageRendererDefinition::CrudTable {
                table: TableDefinition {
                    model_id,
                    page_size: config.page_size,
                },
            })
        }
        PageRendererDefinition::Extension {
            extension_type,
            config,
            ..
        } if extension_type == ExtensionType::of::<AioRuntimePageExtension>() => {
            serde_json::from_value::<AioRuntimePageConfig>(config)
                .map(|config| config.renderer)
                .context("解析 AIO 运行时页面配置失败")
        }
        PageRendererDefinition::Extension {
            extension_type,
            schema_version,
            config,
        } => Ok(studio::PageRendererDefinition::Extension {
            extension_type: extension_type.to_string(),
            schema_version,
            config,
        }),
    }
}

fn resource_catalog(program: &studio::ProgramDefinition) -> ResourceCatalog {
    let resources = program
        .models
        .iter()
        .map(|model| {
            let fields = model
                .fields
                .iter()
                .map(|field| ResourceFieldDefinition {
                    name: field.name.clone(),
                    title: field.title.clone(),
                    kind: resource_field_kind(&field.value_type),
                    required: field.required,
                    list_visible: field.options.list_visible,
                    form_visible: field.name != "id" && field.options.form_visible,
                })
                .collect::<Vec<_>>();
            let fields = std::iter::once(ResourceFieldDefinition {
                name: "id".to_owned(),
                title: "ID".to_owned(),
                kind: resource_field_kind(&model.primary_key.generation.value_type()),
                required: true,
                list_visible: true,
                form_visible: false,
            })
            .chain(fields)
            .collect();
            let id = model.id.to_string();
            (
                id.clone(),
                ResourceDefinition {
                    id,
                    name: model.name.clone(),
                    title: model.title.clone(),
                    id_field: "id".to_owned(),
                    fields,
                    operations: ResourceOperations {
                        list: true,
                        create: true,
                        update: true,
                        delete: true,
                    },
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    ResourceCatalog { resources }
}

fn resource_field_kind(value_type: &studio::ValueType) -> ResourceFieldKind {
    match value_type {
        studio::ValueType::Boolean => ResourceFieldKind::Boolean,
        studio::ValueType::Integer => ResourceFieldKind::Integer,
        studio::ValueType::Decimal => ResourceFieldKind::Decimal,
        studio::ValueType::TimestampMs => ResourceFieldKind::Timestamp,
        studio::ValueType::Text | studio::ValueType::File => ResourceFieldKind::Text,
        studio::ValueType::Optional { value } => resource_field_kind(value),
        studio::ValueType::Any
        | studio::ValueType::Null
        | studio::ValueType::Object { .. }
        | studio::ValueType::List { .. } => ResourceFieldKind::Json,
    }
}

async fn execute_resource(request: ResourceRequest) -> Result<ResourceResponse> {
    match request {
        ResourceRequest::List {
            resource_id,
            page,
            page_size,
        } => {
            let offset = page.saturating_mul(page_size);
            let path =
                format!("/api/runtime/models/{resource_id}/records?o={offset}&s={page_size}");
            let page = get_api::<RuntimeRecordPage>("", &path)
                .await
                .map_err(|error| anyhow!(error))?;
            Ok(ResourceResponse::Page(ResourcePage {
                items: page.d.into_iter().map(resource_record).collect(),
                total: page.t,
            }))
        }
        ResourceRequest::Create {
            resource_id,
            values,
        } => {
            let path = format!("/api/runtime/models/{resource_id}/records");
            let input = RuntimeRecordInput {
                payload: record_payload(values),
            };
            let record = post_api::<_, RuntimeRecordView>("", &path, &input)
                .await
                .map_err(|error| anyhow!(error))?;
            Ok(ResourceResponse::Record(resource_record(record)))
        }
        ResourceRequest::Update {
            resource_id,
            record_id,
            values,
        } => {
            let path = format!("/api/runtime/models/{resource_id}/records/{record_id}");
            let input = RuntimeRecordInput {
                payload: record_payload(values),
            };
            let record = patch_api::<_, RuntimeRecordView>("", &path, &input)
                .await
                .map_err(|error| anyhow!(error))?;
            Ok(ResourceResponse::Record(resource_record(record)))
        }
        ResourceRequest::Delete {
            resource_id,
            record_id,
        } => {
            let path = format!("/api/runtime/models/{resource_id}/records/{record_id}");
            delete_api::<bool>("", &path)
                .await
                .map_err(|error| anyhow!(error))?;
            Ok(ResourceResponse::Deleted)
        }
    }
}

fn resource_record(record: RuntimeRecordView) -> ResourceRecord {
    let mut values = match record.payload {
        Value::Object(values) => values.into_iter().collect::<BTreeMap<_, _>>(),
        _ => BTreeMap::new(),
    };
    values.insert("id".to_owned(), Value::String(record.id));
    values
}

fn record_payload(mut values: ResourceRecord) -> Value {
    values.remove("id");
    Value::Object(values.into_iter().collect::<Map<_, _>>())
}

fn definition_id(id: SymbolId) -> DefinitionId {
    DefinitionId::from_value(id.to_string())
}

fn symbol_id(id: &DefinitionId) -> Result<SymbolId> {
    SymbolId::parse(id.as_str())
}

fn menu_child_count(menus: &[studio::MenuDefinition], id: SymbolId) -> Result<usize> {
    for menu in menus {
        if menu.id == id {
            return Ok(menu.children.len());
        }
        if let Ok(count) = menu_child_count(&menu.children, id) {
            return Ok(count);
        }
    }
    bail!("菜单不存在: {id}")
}

fn unique_route_path(definition: &studio::ProgramDefinition, name: &str) -> String {
    let segment = name.trim_matches('/');
    let base = if segment.is_empty() { "page" } else { segment };
    for suffix in 1_u32..=u32::MAX {
        let path = if suffix == 1 {
            format!("/{base}")
        } else {
            format!("/{base}-{suffix}")
        };
        if definition.routes.iter().all(|route| route.path != path) {
            return path;
        }
    }
    format!("/{base}-{}", SymbolId::new())
}
