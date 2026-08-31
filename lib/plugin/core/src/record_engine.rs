impl EngineExecutor {
    /// 插入记录并执行完整写入管道。
    pub async fn insert_record(
        &self,
        model_name: &str,
        raw_payload: Value,
    ) -> anyhow::Result<DataRecordView> {
        let fields = self.store.list_fields(model_name).await?;
        let payload = value_to_object(raw_payload)?;
        validate_payload(&fields, &payload, false)?;
        let mut values = vec![Value::Object(payload)];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut values)
            .await?;
        let payload = value_to_object(values.remove(0))?;
        validate_payload(&fields, &payload, true)?;
        let record = self
            .store
            .persist_record(model_name, Value::Object(payload))
            .await?;
        Ok(record.into())
    }

    /// 更新记录并执行完整写入管道。
    pub async fn update_record(
        &self,
        model_name: &str,
        record_id: &str,
        raw_payload: Value,
    ) -> anyhow::Result<DataRecordView> {
        let existing = self
            .store
            .get_record(model_name, record_id)
            .await?
            .ok_or_else(|| anyhow!("记录不存在: {model_name}/{record_id}"))?;
        let fields = self.store.list_fields(model_name).await?;
        let mut payload = value_to_object(existing.payload.0)?;
        for (key, value) in value_to_object(raw_payload)? {
            payload.insert(key, value);
        }
        validate_payload(&fields, &payload, false)?;
        let mut values = vec![Value::Object(payload)];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut values)
            .await?;
        let payload = value_to_object(values.remove(0))?;
        validate_payload(&fields, &payload, true)?;
        let record = self
            .store
            .replace_record_payload(model_name, record_id, Value::Object(payload))
            .await?;
        Ok(record.into())
    }

    /// 查询单条记录并执行 computed 字段求值。
    pub async fn get_record(
        &self,
        model_name: &str,
        record_id: &str,
    ) -> anyhow::Result<DataRecordView> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        let record = self
            .store
            .get_record(model_name, record_id)
            .await?
            .ok_or_else(|| anyhow!("记录不存在: {model_name}/{record_id}"))?;
        let mut payloads = vec![record.payload.0.clone()];
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, &fields, &mut payloads)
            .await?;
        let payload = payloads.remove(0);
        Ok(DataRecordView {
            id: record.id,
            model_name: record.model_name,
            payload,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        })
    }

    /// 删除记录。
    pub async fn delete_record(&self, model_name: &str, record_id: &str) -> anyhow::Result<()> {
        self.store.delete_record(model_name, record_id).await
    }

    /// 查询记录并执行集合化计算字段。
    pub async fn list_records(
        &self,
        model_name: &str,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        let raw = self.store.list_raw_records_page(model_name, page).await?;
        self.evaluate_record_page(model_name, &fields, raw).await
    }

    /// 按结构化条件查询记录并执行集合化计算字段。
    pub async fn list_records_with_criteria(
        &self,
        model_name: &str,
        criteria: &RecordCriteria,
        page: PageParams,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        self.store.ensure_model(model_name).await?;
        let fields = self.store.list_fields(model_name).await?;
        for filter in criteria.all.iter().chain(&criteria.any) {
            if !fields.iter().any(|field| field.name == filter.field) {
                bail!("筛选字段不存在: {model_name}.{}", filter.field);
            }
            if filter.value.is_empty() {
                bail!("筛选值不能为空: {model_name}.{}", filter.field);
            }
        }
        let sort_field_type = match &criteria.sort {
            Some(sort) => Some(
                fields
                    .iter()
                    .find(|field| field.name == sort.field)
                    .with_context(|| format!("排序字段不存在: {model_name}.{}", sort.field))?
                    .field_type
                    .as_str(),
            ),
            None => None,
        };
        let raw = self
            .store
            .list_raw_records_page_with_criteria(model_name, criteria, sort_field_type, page)
            .await?;
        self.evaluate_record_page(model_name, &fields, raw).await
    }

    async fn evaluate_record_page(
        &self,
        model_name: &str,
        fields: &[MetaField],
        raw: PageData<DataRecord>,
    ) -> anyhow::Result<PageData<DataRecordView>> {
        let mut payloads = raw
            .d
            .iter()
            .map(|record| record.payload.0.clone())
            .collect::<Vec<_>>();
        BatchComputedEvaluator::new(self.store.clone())
            .evaluate(model_name, fields, &mut payloads)
            .await?;
        let rows = raw
            .d
            .into_iter()
            .zip(payloads)
            .map(|(record, payload)| DataRecordView {
                id: record.id,
                model_name: record.model_name,
                payload,
                created_at_ms: record.created_at_ms,
                updated_at_ms: record.updated_at_ms,
            })
            .collect::<Vec<_>>();
        Ok(PageData {
            d: rows,
            t: raw.t,
            p: raw.p,
        })
    }
}

impl BatchComputedEvaluator {
    /// 创建集合化计算字段求值器。
    pub fn new(store: RecordStore) -> Self {
        Self { store }
    }

    /// 对一批 payload 执行 computed 字段求值。
    pub async fn evaluate(
        &self,
        model_name: &str,
        fields: &[MetaField],
        records: &mut [Value],
    ) -> anyhow::Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let computed_fields = fields
            .iter()
            .filter(|field| field.field_type == "computed")
            .collect::<Vec<_>>();
        if computed_fields.is_empty() {
            return Ok(());
        }
        let dependencies = self
            .load_dependency_cache(model_name, &computed_fields, records)
            .await?;
        for (record_index, record) in records.iter_mut().enumerate() {
            let payload = record
                .as_object_mut()
                .ok_or_else(|| anyhow!("计算字段只能处理 JSON 对象记录"))?;
            for field in &computed_fields {
                let Some(expression) = field.expression.as_deref() else {
                    continue;
                };
                let deps = match dependencies
                    .get(&field.id)
                    .and_then(|items| items.get(record_index))
                {
                    Some(value) => value.clone(),
                    None => HashMap::new(),
                };
                let value = evaluate_expression(expression, payload, &deps)
                    .with_context(|| format!("计算字段失败: {}.{}", model_name, field.name))?;
                payload.insert(field.name.clone(), eval_value_to_json(value));
            }
        }
        Ok(())
    }

    async fn load_dependency_cache(
        &self,
        _model_name: &str,
        computed_fields: &[&MetaField],
        records: &[Value],
    ) -> anyhow::Result<HashMap<String, Vec<HashMap<String, Value>>>> {
        let mut output = HashMap::new();
        for field in computed_fields {
            let deps = parse_dependencies(field.dependency_json.as_deref())?;
            if deps.is_empty() {
                continue;
            }
            let mut aliases_by_record = vec![HashMap::new(); records.len()];
            for dep in deps {
                let cache = self.load_one_dependency(&dep, records).await?;
                for (record_index, record) in records.iter().enumerate() {
                    let Some(object) = record.as_object() else {
                        continue;
                    };
                    let Some(record_id) = object.get(&dep.local_field).and_then(Value::as_str)
                    else {
                        continue;
                    };
                    if let Some(source_payload) = cache.get(record_id)
                        && let Some(value) = source_payload.get(&dep.source_payload_field)
                    {
                        aliases_by_record[record_index].insert(dep.alias.clone(), value.clone());
                    }
                }
            }
            output.insert(field.id.clone(), aliases_by_record);
        }
        Ok(output)
    }

    async fn load_one_dependency(
        &self,
        dependency: &ComputedDependency,
        records: &[Value],
    ) -> anyhow::Result<HashMap<String, Map<String, Value>>> {
        let ids = records
            .iter()
            .filter_map(Value::as_object)
            .filter_map(|object| object.get(&dependency.local_field))
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let wanted = ids.into_iter().collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT id, model_name, payload, created_at_ms, updated_at_ms
             FROM engine_data_records WHERE model_name = $1 AND id = ANY($2)",
        )
        .bind(&dependency.source_model_name)
        .bind(&wanted)
        .fetch_all(&self.store.pool)
        .await
        .with_context(|| {
            format!(
                "批量加载 computed 依赖失败: {}",
                dependency.source_model_name
            )
        })?;
        let mut cache = HashMap::new();
        for row in &rows {
            let row = data_record_from_row(row)?;
            let payload = value_to_object(row.payload.0)?;
            cache.insert(row.id, payload);
        }
        Ok(cache)
    }
}
