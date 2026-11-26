# 人工集成测试指引

本文带你把编译服务放到真实环境里跑一遍：真实的集群、真实的数仓、真实的模型。
按章节顺序执行，每一步都写明**前置条件、要执行的命令、预期看到什么、不符合时怎么排查、需要记录什么**。

测试分两条轨道：

- **轨道 A（基准数据）**：把仓库自带的财险示例数据装进测试用的 schema，跑一套有标准答案的问题。
  结果是确定的"通过/失败"，用来回答"内核在这个环境里对不对"。
- **轨道 B（真实模型）**：用你方自己的模型和查询，与现有报表或手写 SQL 的结果人工比对。
  用来回答"内核对我们的业务对不对"。

配套的命令行工具 `ppcube-itest` 在 [tools/integration-kit](../tools/integration-kit)。
它做的事和调用方的后端一样：向服务请求编译，把 SQL 原样拿到引擎上执行，参数单独绑定，
再把结果与预期比对并生成报告。**内核本身从不执行 SQL，也从不连接数仓。**

> 本文中标注"【未在本仓库验证】"的步骤依赖你方的集群和发布流程，编写时无法实际执行；
> 其余命令都在本地环境（服务 + Presto 0.296 / Doris 2.1.8 / Druid 26.0.0 容器）上逐条执行过。

---

## 0. 环境信息表

开始之前先填好这张表。后面每一章都会用到，测试结束后它也是报告的第一页。

| 项目 | 你方的值 | 说明 |
| --- | --- | --- |
| 部署平台 | | Kubernetes 或其他；命名空间 |
| 服务地址 | | gRPC 端口，如 `ppcube.xxx:50051` |
| 服务证书的 CA | | PEM 文件路径；用公共 CA 时留空 |
| 读 token 来源 | | 存放 token 的文件；**不要把 token 写进任何文档** |
| 管理 token 来源 | | 仅第 7 章需要时使用 |
| 模型存储位置 | | NAS 目录，或 `s3://桶/前缀` |
| 模型刷新周期 | | 服务的 `--model-refresh-seconds`，默认 30 秒 |
| Presto 地址 / 版本 / catalog | | 生产为 0.296 |
| Doris FE 地址 / 版本 | | 生产为 2.1.8；新建表需要的副本数 |
| Druid Router 地址 / 版本 | | 生产为 26.0 |
| Druid NULL 处理模式 | | `druid.generic.useDefaultValueForNull`；第 3 章的 `env` 会读出来 |
| Druid merge buffer 数 | | `druid.processing.numMergeBuffers` |
| 测试用 schema（Presto / Doris） | | 建议专用，如 `ppcube_itest` |
| 测试用 datasource 前缀（Druid） | | 默认 `ppcube_itest_` |
| 各引擎的数据源名 | | 服务启动参数 `--data-source <名字>=<方言>` 里的名字 |

**关于写权限**：轨道 A 需要在每个引擎上建 9 张小表（共 54 行）。如果某个引擎不允许写入，
该引擎只做轨道 B；或者用 `ppcube-itest load --dry-run` 打印建表语句，交给有权限的人执行。

工具对引擎的改动范围是固定的：只创建、替换或删除名为 `ins_*`（加上你配置的前缀）的 9 张表，
只在你配置的 schema 里。schema 不存在时会创建，但从不删除 schema。

---

## 1. 取得并验证发布物【未在本仓库验证】

**前置条件**：仓库 `main` 上最新提交的 `t0`、`image`、`aarch64` 检查为绿色。

发布由仓库所有者打 tag 触发，流程见 [docs/agents/release.md](agents/release.md)：

```sh
git switch main && git pull --ff-only
git tag v0.2.0 && git push origin v0.2.0
```

**预期**：GitHub Actions 的 `release` workflow 四个 job 全部成功；仓库出现 Release `v0.2.0`，
其说明末尾有镜像地址和摘要（`sha256:…`）。

> 这是 `publish` job 的**第一次真实运行**（推送镜像、合成多架构、签名、创建 Release）。
> 如果中途失败，按 `docs/agents/release.md` 的"When it fails"一节处理，并把失败的 job 和日志记录下来。

部署前验证镜像的来源证明：

```sh
gh attestation verify oci://ghcr.io/<owner>/<repository>@sha256:<摘要> --repo <owner>/<repository>
```

**预期**：输出验证通过。首次推送后，package 的可见性需要仓库所有者在 GitHub 上设置（公开，或授予部署凭据读取权限）。

**记录**：镜像摘要；`release` workflow 的运行链接。

---

## 2. 部署 3 副本【未在本仓库验证】

**前置条件**：第 1 章的镜像摘要；证书、读 token、管理 token；模型存储已可被集群挂载或访问。

按 [docs/deployment.md](deployment.md) 和 [deploy/kubernetes.yaml](../deploy/kubernetes.yaml) 部署。需要你方提供三样东西：

1. 把清单里的镜像 tag 换成第 1 章验证过的摘要。
2. 创建 Secret：

   ```sh
   kubectl create secret generic <secret 名> \
     --from-file=tls.crt=server.pem --from-file=tls.key=server.key \
     --from-file=read.token --from-file=admin.token
   ```

3. 把清单里的模型存储卷换成你方的 PVC；用 S3 时按 `docs/deployment.md` 改 `--model-store` 和环境变量。

另外，为模型用到的**每个数据源**写一行 `--data-source <名字>=<方言>`。本测试建议三个引擎各用一个名字，
并与 `itest.ini` 里的 `data_source` 保持一致。

**预期**：3 个 Pod 全部 Ready。逐个副本检查指标（metrics 端口不要暴露到集群外）：

```sh
kubectl port-forward pod/<pod 名> 9100:9100 &
curl -s localhost:9100/metrics | grep -E 'ppcube_build_info|ppcube_model_versions'
```

每个副本的 `ppcube_build_info` 应显示 `kernel_version="0.2.0"`；`ppcube_model_versions` 是当前可用的模型版本数。

**排查**：Pod 不 Ready 时看日志。服务启动失败会在 stderr 写明原因后退出；
模型存储列不出来会直接退出；某个模型版本有问题不会阻止启动，只会被拒绝并记录（见第 7 章）。

**记录**：3 个 Pod 的名字和启动时间；`ppcube_build_info` 的输出。

---

## 3. 安装工具与连通性冒烟

**前置条件**：一台能同时访问服务和各引擎的机器；Python 3.9 或更高；仓库的一份检出。

```sh
cd <仓库检出目录>
python3 -m venv .itest && . .itest/bin/activate
pip install --upgrade pip
pip install clients/python tools/integration-kit
cp tools/integration-kit/itest.example.ini itest.ini
```

`pip` 必须先升级：Python 3.9 自带的旧版 pip 会在构建客户端时找不到服务契约。

编辑 `itest.ini`，按第 0 章的表填写。要点：

- `[service]` 写服务地址、CA 文件、读 token 文件。
- 每个引擎一个 `[engine:<名字>]` 小节。只测一个引擎就只留一个小节。
- **密码和 token 都不写进这个文件**：密码写环境变量名（`password_env = DORIS_PASSWORD`），执行前 `export`；token 写文件路径。
- `model_version` 是本次测试要发布的模型版本，形如 `itest/presto/v1`：业务线 `itest/presto`，版本 `v1`。
- `data_source` 必须等于服务启动参数里该引擎的数据源名。

此时模型还没发布，先只检查引擎：

```sh
ppcube-itest env
```

**预期**：每个引擎打印版本号。Druid 还会打印 NULL 处理模式，把它填回第 0 章的表。
这个值读自 Druid 进程自己的运行时属性（`/status/properties`），不是靠查询猜的；账号无权读取时显示 `unknown`，需要向 Druid 管理员确认。

```text
engine druid (druid)
  model    FAILED: NOT_FOUND …            ← 模型还没发布，此时属于正常
  engine   26.0.0, NULL handling sql-compatible (druid.generic.useDefaultValueForNull=false)
  tables   0 of 9 benchmark tables in druid
```

此时命令最后一行是 `env: FAILED`，原因只有模型未发布；第 4.1 步之后应当变为 `env: PASSED`。

**排查**：

| 现象 | 原因 |
| --- | --- |
| `the call to the service failed: UNAVAILABLE` | 服务地址不通，或 CA 文件不对。本机设置了 `http_proxy`/`https_proxy` 时，gRPC 可能经代理去连服务：把服务主机加入 `no_proxy` |
| `UNAUTHENTICATED` | token 文件内容不对；注意它必须是读 token 或管理 token |
| `engine FAILED: …` | 引擎地址、账号或网络问题；消息是引擎自己的原文 |
| `password_env names X, which is not set` | 忘了 `export X=…` |

**记录**：`ppcube-itest env` 的完整输出。

---

## 4. 轨道 A：基准数据

对每个引擎重复 4.1–4.4。下面以 `presto` 为例，把 `--engine` 换成 `doris`、`druid` 即可。

### 4.1 发布基准模型

```sh
ppcube-itest model --engine presto --out ./itest-reports/store
```

它把示例模型改写成指向你的测试 schema、你的数据源名（Druid 还会把时间列改成 `__time`），
并按模型存储的目录布局写出来：

```text
./itest-reports/store/itest/presto/models/v1/manifest.json
                                             /cubes/<cube>.yml  （9 个）
                                             /views/<view>.yml  （2 个）
```

把 `./itest-reports/store/` **下面的目录树**复制到模型存储的根（NAS 目录或 S3 前缀），保持层级不变。
**`manifest.json` 最后复制**：服务只在看到 manifest 之后才提供这个版本。

等一个刷新周期（默认 30 秒），然后：

```sh
ppcube-itest env --engine presto
```

**预期**：`model` 一行显示版本、摘要、内核版本和 `11 cubes and views`。

**排查**：仍是 `NOT_FOUND` 时，看任一副本的日志里是否有
`{"event":"model_store","model_version":"itest/presto/v1","outcome":"rejected","reason":…}`。
`reason` 的含义：`data-source` 是服务没有配置这个数据源名；`layout` 是目录层级不对；
`files` 是某个文件不是合法的 cube 或 view 文件（YAML 语法错误、文件名与内容不符等）；`model` 是文件都合法但模型整体校验不通过。

### 4.2 装载基准数据

```sh
ppcube-itest load --engine presto
```

**预期**：9 行 `loaded …`，最后 `load: PASSED, 9 tables`。Druid 上每张表是一个 SQL 摄入任务，9 张表共需几分钟。

- 表已存在时工具**不做任何改动**并提示；确认要重装才加 `--replace`。
- 没有建表权限时：`ppcube-itest load --engine presto --dry-run > load.sql`，把语句交给有权限的人执行。
  Druid 的语句要作为 SQL 摄入任务提交到 `/druid/v2/sql/task`。
- Doris 的副本数用 `itest.ini` 里的 `replication_num` 控制。
- Druid 需要已加载多阶段查询扩展（`druid-multi-stage-query`），账号需要有外部数据源和写 datasource 的权限。

### 4.3 手工看一条查询

先亲眼看一次"请求 → SQL → 结果"。新建 `q.json`：

```json
{"measures": ["policies.written_premium"],
 "dimensions": ["products.line_of_business"],
 "order": [["products.line_of_business", "asc"]]}
```

```sh
ppcube-itest compile --engine presto --query q.json
ppcube-itest run --engine presto --query q.json
```

`compile` 打印：发给服务的请求、编译出的 SQL、参数、会话属性、结果列、fingerprint、模型摘要和内核版本。
`run` 在此之外再到引擎上执行并打印结果。

**预期**（三个引擎相同）：

```text
products.line_of_business  policies.written_premium
LIABILITY                  10,500
MOTOR                      5,700
PROPERTY                   2,700
```

### 4.4 跑基准套件

```sh
ppcube-itest suite --engine presto --builtin insurance --report ./itest-reports/presto
```

套件是 4 类用户（核保、理赔、财务、东区经理）的 17 个问题，东区经理的问题带行级规则。
每个问题一行结果，状态含义：

| 状态 | 含义 | 是否算通过 |
| --- | --- | --- |
| `PASS` | 结果与标准答案逐行一致 | 是 |
| `REFUSED` | 内核按预期不给 SQL（类别相符） | 是 |
| `SHOWN` | 没有标准答案，结果已列出，需人工判断 | 需人工 |
| `FAIL` | 结果与标准答案不一致，或该拒绝的没拒绝、不该拒绝的被拒绝 | 否 |
| `ERROR` | 调用服务失败，或引擎拒绝执行这条 SQL | 否 |

**预期**：见下表。最后一行 `suite: PASSED`，命令退出码为 0。

| 引擎 | 预期 |
| --- | --- |
| Presto 0.296 | 15 个 `PASS`，2 个 `REFUSED` |
| Doris 2.1.8 | 16 个 `PASS`，1 个 `REFUSED` |
| Druid 26.0 | 15 个 `PASS`，2 个 `REFUSED` |

各引擎"按预期拒绝"的问题不同，这是方言差异，不是缺陷，完整清单在
[tools/integration-kit/suites/insurance-dialects.json](../tools/integration-kit/suites/insurance-dialects.json)。

**排查**：

- `ERROR … the engine refused the SQL`：打开 `report.md` 找到这条用例，里面有完整 SQL、参数和引擎的报错原文。
  这是最需要反馈的一类问题，请把这一节整段贴回来。
- `FAIL … row N, <列>: 实际值, expected 期望值`：先确认 4.2 的数据是本工具装的、没有被别人改过（`load --replace` 重装）。
- Druid 在默认（legacy）NULL 模式下，`chief.underwriter/uw_renewal_share_by_branch` 的结果会与标准答案不同：
  该模式把 NULL 数值存成 0，`IS NOT NULL` 对每一行都成立。这是引擎配置差异，记录即可。

**记录**：每个引擎的 `report.md` 和 `report.json`（报告里没有 token 和密码，可以直接归档）。

---

## 5. 轨道 B：真实模型

**前置条件**：你方的一个 Cube 模型（按 cube.dev 的目录习惯，一个 cube 或 view 一个文件）；
若干有代表性的查询；每个查询有一个可比对的"已知正确结果"（现有报表，或手写 SQL 的结果）。

### 5.1 发布真实模型

真实模型由语义层发布。布局如下，细节见 [服务指南](../crates/pp-cube-service/README.md) 的"From shared storage"：

```text
<业务线>[/<子业务线>[/<子业务线>]]/models/<版本>/manifest.json
                                                  /cubes/<cube>.yml
                                                  /views/<view>.yml
```

发布前可以先校验模型（`ValidateModel` 接口）；模型有问题时版本会被拒绝，原因见 4.1 的排查方法。

在 `itest.ini` 里为真实模型加一个引擎小节（同一个引擎可以有多个小节，名字不同即可）：

```ini
[engine:presto-real]
dialect = presto
host = …
catalog = …
schema = <真实数据所在的 schema，仅作为连接默认值>
data_source = <模型里的 data_source>
model_version = <业务线>/<版本>
```

`ppcube-itest meta --engine presto-real` 列出这个版本的全部 cube、view、度量、维度。先确认成员齐全。

> 对真实模型的小节**不要执行 `load` 和 `clean`**：它们只认基准表名，但没有必要在真实 schema 上冒险。

### 5.2 编写自己的套件

复制 [tools/integration-kit/suites/template.json](../tools/integration-kit/suites/template.json)，每个查询一条用例：

- 只写 `query`：结果列出来，由人与报表比对（状态 `SHOWN`）。
- 加 `expect`：工具逐行核对。列名用内核的列别名（先 `compile` 一次看 `columns`），时间写成 `{"timestamp": "2026-01-01T00:00:00"}`。
- 加 `rowFilters`：在这些行级规则下编译。
- 加 `refused`：声明这条查询应当得不到 SQL，以及类别。

建议覆盖：单事实聚合、带过滤的度量、跨事实查询、时间粒度与时区、滚动窗口或期初至今、同比环比、
分页、明细（ungrouped）、视图，以及你方最常用的 3–5 张报表。

### 5.3 执行与比对

```sh
ppcube-itest compile --engine presto-real --query <某个查询>.json     # 先审 SQL
ppcube-itest suite --engine presto-real --file my-suite.json --report ./itest-reports/presto-real
```

对每条 `SHOWN` 的用例，在 `report.md` 里把结果与已知正确结果比对，并审一遍 SQL：
表和关联是否符合预期、过滤是否都在、有没有明显低效的写法。

**记录**：每条用例的结论（一致 / 不一致 / 被拒绝及原因），不一致的附上两边的结果和 SQL。
被拒绝的查询请记下内核给出的原因原文，它是后续功能排期的直接输入。

---

## 6. 行级安全

**前置条件**：轨道 A 已完成。以下用基准模型演示，真实模型上请用你方真实的规则再做一遍。

行级规则由**调用方的后端**根据用户身份决定，通过安全过滤传给内核；内核把它们加在查询之上，查询无法放宽。

新建 `by-region.json`：

```json
{"measures": ["policies.policy_count"],
 "dimensions": ["branches.region_name"],
 "order": [["branches.region_name", "asc"]]}
```

新建 `east.json`（一条安全过滤）：

```json
[{"member": "branches.region_name", "operator": "equals", "values": ["East"]}]
```

```sh
ppcube-itest run --engine presto --query by-region.json
ppcube-itest run --engine presto --query by-region.json --filters east.json
```

**预期**：第一条返回 `East 4`、`North 5` 两行；第二条只返回 `East 4`，且 `parameters` 里出现 `"East"`（参数绑定，没有拼进 SQL）。

再验证查询无法放宽规则。新建 `widen.json`，在查询里用 `or` 试图把 North 加回来：

```json
{"measures": ["policies.policy_count"],
 "dimensions": ["branches.region_name"],
 "filters": [{"or": [
   {"member": "branches.region_name", "operator": "equals", "values": ["North"]},
   {"member": "branches.region_name", "operator": "equals", "values": ["East"]}]}],
 "order": [["branches.region_name", "asc"]]}
```

```sh
ppcube-itest run --engine presto --query widen.json --filters east.json
```

**预期**：仍然只有 `East 4`。SQL 的 `WHERE` 中，安全过滤是顶层的 `AND` 条件，`or` 组在它旁边而不是包住它。

**记录**：三条命令的输出。

---

## 7. 模型版本生命周期

**前置条件**：对模型存储有写权限；知道刷新周期。

| 步骤 | 操作 | 预期 |
| --- | --- | --- |
| 发布新版本 | 把 `itest.ini` 里的 `model_version` 改成 `itest/presto/v2`，重新执行 4.1 的 `model` 和复制 | 一个刷新周期后 `env` 显示 v2；**v1 仍然可用**（把配置改回 v1 再 `env` 确认） |
| 三副本一致 | 逐个副本看 `ppcube_model_versions` | 三个副本的数值相同。副本各自读取存储，最多相差一个刷新周期 |
| 版本不可变 | 修改已发布版本目录里的某个文件 | 运行中的服务不会重新读取已加载的版本（`meta` 看不到改动）；要改就发新版本。**不要这样改生产版本**：之后启动的副本会读到改过的内容 |
| 坏版本 | 发布一个 `itest/presto/bad`：复制 v1，把某个 cube 文件改成非法 YAML（如只留 `cubes: [`） | 该版本被拒绝，日志出现 `"outcome":"rejected","reason":"files"`，指标 `ppcube_model_store_rejected_versions{reason="files"}` 为 1；**其他版本不受影响** |
| 下线 | 删除某个版本的 `manifest.json` | 下一个刷新周期后该版本不可用，对它的编译返回 `NOT_FOUND` |
| 存储不可达 | （可选）暂时断开存储 | 运行中的副本保留已加载的版本，`ppcube_model_store_last_scan_failed` 为 1；**此时不要重启副本**，新副本会因列不出存储而启动失败 |

**记录**：每一步的时间点和观察到的指标值。

---

## 8. 多副本、升级与回滚【未在本仓库验证】

**前置条件**：3 个副本在运行；有一个持续发请求的终端。

持续发请求（每秒一次，观察有没有失败）：

```sh
while true; do ppcube-itest compile --engine presto --query q.json > /dev/null && echo ok || echo FAILED; sleep 1; done
```

| 步骤 | 操作 | 预期 |
| --- | --- | --- |
| 失去一个副本 | `kubectl delete pod <某个 pod>` | 请求基本连续；个别请求可能 `UNAVAILABLE`，调用方重试即可；Pod 被自动补回 |
| 滚动重启 | `kubectl rollout restart deployment/<名字>` | 一次只换一个副本，始终至少 2 个可用；被替换的副本先处理完在途请求再退出 |
| 升级 | 换成新版本镜像的摘要 | 滚动完成后每个副本的 `ppcube_build_info` 都是新版本。**同一请求的 fingerprint 会变**，这是预期：内核版本是 fingerprint 的一部分 |
| 回滚 | `kubectl rollout undo deployment/<名字>` | 回到旧版本；没有状态需要迁移，模型存储未被写入 |

注意 gRPC 是长连接：经由普通 Service 时，一个连接固定落在一个副本上。要看到请求分散到多个副本，
调用方需要开多个连接或在客户端做负载均衡，见 `docs/deployment.md` 的"Balancing"。

目前只有 0.2.0 一个发布版本，升级和回滚可以先用同一个镜像做一次滚动来演练流程，等下一个版本发布后再做真实升级。

**记录**：每一步期间失败请求的数量和错误码。

---

## 9. 各引擎专项

### Presto

- **请求头大小**。预编译语句通过 HTTP 头传输，JDBC 和 Python 驱动都是如此。带同比、多维度的大查询编码后可能超过服务端默认的 8KB，
  表现为执行前就收到 HTTP 错误。真实模型上最长的那几条 SQL 要专门跑一次；超限时调大 `http-server.max-request-header-size`。
- **分页**。带 `offset` 的查询，内核会在会话属性里返回 `offset_clause_enabled=true`，执行方必须设置。
  基准套件里的 `chief.underwriter/uw_branches_second_page` 验证了这一点。
- **按周、按季度的滚动窗口**在 Presto 上被拒绝（`UNSUPPORTED`），因为 Presto 没有这两个单位的 interval。

### Doris

- **占位符**。内核写 `?`，MySQL 协议的 Python 驱动要 `%s`。工具用客户端的 `sql_as('format')` 转换，
  它不会动引号里的内容。自己写后端时用同一个方法，不要手工替换。
- **滚动窗口**的时间段是逐个写出来的，所以按粒度滚动必须给日期范围。

### Druid

- **查询上下文**。每个 Druid 结果都带 `useApproximateCountDistinct=false`，执行方必须作为查询上下文传入，否则 `COUNT(DISTINCT)` 是估算值。
- **NULL 模式**。见 4.4 的说明；第 3 章 `env` 的输出里有当前取值。
- **merge buffer**。同一查询里两个精确去重计数至少需要 3 个 merge buffer。
- **多事实查询和滚动窗口在 broker 上合并**。每个聚合是一个子查询，受 `druid.server.http.maxSubqueryRows`（默认 10 万）限制；
  滚动窗口还会先做"时间段数 × 聚合行数"的笛卡尔积。真实数据上要专门挑一个分组数大的多事实查询、一个日期范围长的滚动查询各跑一次，
  记录耗时和是否触发限制。
- **明细查询**（`ungrouped`）在 Druid 上被拒绝。
- 模型约定：表名写 `druid.<datasource>`，时间维度写 `{CUBE}.__time`。

**记录**：每一项的实际表现；触发限制时的报错原文。

---

## 10. 拒绝与错误

逐类触发一次，确认调用方拿到的东西符合预期。

### 10.1 服务答复了，但没有 SQL

这类情况**重试没有意义**。工具打印 `no SQL (<类别>): <原因>`，`compile`/`run` 的退出码为 3。
原因只包含模型成员名和规则，不包含 SQL 和表名，可以直接展示给写查询的人。

| 类别 | 怎么触发（基准模型） | 预期原因 |
| --- | --- | --- |
| `QUERY` | `{"measures": ["policies.no_such_measure"]}` | 指出该度量不在模型里 |
| `QUERY` | `{"measures": ["earned_premiums.paid_loss_ratio"], "dimensions": ["products.line_of_business"]}` | 指出这些 cube 之间没有连接路径 |
| `UNSUPPORTED` | 在 Presto 上按季度查 `policies.written_premium_ytd`（套件里已有） | 指出 Presto 不能按季度生成时间序列 |
| `UNSUPPORTED` | 在 Druid 上 `{"ungrouped": true, "dimensions": ["policies.policy_no"]}` | 指出 Druid 不支持 ungrouped |

### 10.2 调用本身失败

工具打印 `the call to the service failed: <状态码>`，退出码为 2。

| 状态码 | 怎么触发 | 调用方应当 |
| --- | --- | --- |
| `NOT_FOUND` | 把 `model_version` 改成不存在的版本 | 检查版本名；不要重试 |
| `UNAUTHENTICATED` | 把 token 文件内容改错 | 检查凭据；不要重试 |
| `RESOURCE_EXHAUSTED` | 并发超过服务的 `--workers` + `--queue` | 退避后重试，或扩容 |
| `UNAVAILABLE` | 服务地址不通，或副本正在替换 | 重试一次 |
| `DEADLINE_EXCEEDED` | 编译超过服务的 `--compile-timeout-ms` | 重试一次；持续出现则上报这条查询 |

### 10.3 SQL 编译出来了，但引擎拒绝执行

套件里表现为 `ERROR … the engine refused the SQL: <引擎原文>`。**这是集成测试最想找到的问题**：
内核认为可以编译，引擎却不接受。请连同 `report.md` 里该用例的整节反馈。

**记录**：每一类的实际输出。

---

## 11. 测试记录与结论

### 11.1 清理

测试结束后删除基准表（真实模型的小节不需要）：

```sh
ppcube-itest clean --engine presto --yes
```

Druid 上是把 datasource 标记为不再使用，段文件由 Druid 自己的清理任务回收。
模型存储里的 `itest/…` 目录可以删除 manifest 让版本下线，或整个目录移除。

### 11.2 逐项记录

| 章 | 项目 | 结果（通过 / 失败 / 未执行） | 备注或问题编号 |
| --- | --- | --- | --- |
| 1 | 发布与镜像验证 | | |
| 2 | 3 副本部署 | | |
| 3 | 工具安装与连通性 | | |
| 4 | 轨道 A：Presto | | |
| 4 | 轨道 A：Doris | | |
| 4 | 轨道 A：Druid | | |
| 5 | 轨道 B：真实模型（每个引擎一行） | | |
| 6 | 行级安全 | | |
| 7 | 模型版本生命周期 | | |
| 8 | 多副本、升级与回滚 | | |
| 9 | 各引擎专项 | | |
| 10 | 拒绝与错误 | | |

### 11.3 问题清单

每个问题一条，至少包含：

- 现象，以及它出现在哪一章哪一步；
- 引擎和版本、模型版本和摘要、内核版本（`report.md` 开头都有）；
- 查询 JSON、安全过滤、编译出的 SQL 和参数（`report.md` 对应用例的整节）；
- 实际结果与期望结果，或引擎的报错原文。

**不要**在问题里贴 token、密码、内部主机名或真实业务数据；仓库是公开的。真实模型的问题请先脱敏，
或改用基准模型复现。

### 11.4 结论

- 环境信息表（第 0 章，已填）
- 轨道 A：三个引擎各自的通过数 / 总数
- 轨道 B：比对的查询数，一致数，不一致数，被拒绝数及原因分类
- 阻塞上线的问题
- 不阻塞但需要跟进的问题
- 结论：可以上线 / 修复后复测 / 需要进一步评估
