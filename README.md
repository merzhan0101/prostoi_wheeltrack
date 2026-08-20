# Система учёта простоев оборудования

Веб-приложение для создания, просмотра, фильтрации и редактирования простоев оборудования производственных участков колесного производства.

Приложение интегрируется с внешней системой ТОИР, использует Microsoft SQL Server для локальных справочников и данных, а также предоставляет единый интерфейс для нескольких производственных участков.

## Поддерживаемые участки

| Участок | Slug | DepartmentID | Place ID | Маршрут |
|---|---|---|---:|---|
| Заготовительный участок КПЦ | `workpiece` | `92b138d1-c131-11ed-8163-00155d16fe68` | 1 | `/downtimes/workpiece` |
| Прессопрокатный участок | `rolling` | `92b138d4-c131-11ed-8163-00155d16fe68` | 4 | `/downtimes/rolling` |
| Участок термообработки колес | `utk` | `92b138e6-c131-11ed-8163-00155d16fe68` | 7 | `/downtimes/utk` |
| Участок полнопрофильной обработки КПЦ | `uppo` | `92b138cc-c131-11ed-8163-00155d16fe68` | 10 | `/downtimes/uppo` |
| Участок неразрушающего контроля КПЦ | `unk` | `92b138e4-c131-11ed-8163-00155d16fe68` | 5 | `/downtimes/unk` |

Конфигурация участков находится в `src/areas.rs`.

## Возможности

- единая страница журнала для всех производственных участков;
- получение последних простоев из ТОИР;
- фильтрация простоев по участку и дате;
- поиск оборудования только в пределах выбранного участка;
- поиск мастеров по `Place_id`;
- создание нового простоя;
- создание простоя для нескольких единиц оборудования;
- выбор нескольких ответственных служб;
- редактирование существующего простоя;
- получение названий оборудования, подразделений и типов простоев из локальных справочников;
- локальное сохранение данных, полученных после успешной отправки в ТОИР.

## Технологии

### Backend

- Rust;
- Actix Web;
- Tokio;
- Tiberius;
- bb8 и bb8-tiberius;
- Reqwest;
- Serde и serde_json;
- Askama;
- Chrono;
- UUID;
- dotenv.

### Frontend

- HTML5;
- CSS3;
- JavaScript;
- Font Awesome;
- Askama templates.

### Базы данных

- Microsoft SQL Server;
- база `WheelTrack`;
- база `Downtime`.

## Архитектура

```text
Браузер
   ↓
Actix Web
   ├── Askama → HTML-страница участка
   ├── API оборудования и сотрудников → MS SQL
   └── API простоев → ТОИР
                         ↓
                 Ответ в формате JSON
                         ↓
             Локальные справочники MS SQL
                         ↓
                Подготовленный JSON
                         ↓
                   Таблица браузера
```

Один backend обслуживает все участки. Участок определяется по параметру `{area}` в URL, после чего backend получает его `DepartmentID` и `Place ID` из `src/areas.rs`.

## Структура проекта

```text
.
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── src
│   ├── main.rs
│   ├── lib.rs
│   ├── areas.rs
│   ├── data_base
│   │   ├── mod.rs
│   │   ├── db_downtimes.rs
│   │   └── id_search.rs
│   ├── infra
│   │   ├── mod.rs
│   │   ├── downtimes.rs
│   │   └── struct_template.rs
│   └── structures
│       ├── mod.rs
│       ├── struct_downtimes.rs
│       └── struct_pool_client.rs
├── templates
│   └── downtimes.html
└── static
    ├── css
    │   └── downtimes.css
    ├── js
    │   └── downtimes.js
    └── ...
```

## Настройка

### Требования

- Rust с Cargo;
- Microsoft SQL Server;
- доступ к базам `WheelTrack` и `Downtime`;
- доступ к API ТОИР.

### Переменные окружения

Создайте файл `.env` в корне проекта:

```env
CONNECTION_STRING=server=tcp:SERVER,1433;database=WheelTrack;user=USER;password=PASSWORD;TrustServerCertificate=true
CONNECTION_RAPORT=server=tcp:SERVER,1433;database=Downtime;user=USER;password=PASSWORD;TrustServerCertificate=true

TOIR_URL=https://toir.example.local
TOIR_LOGIN=login
TOIR_PASSWORD=password
```

Не добавляйте `.env` в Git:

```gitignore
.env
```

## Запуск

```bash
cargo run
```

По умолчанию сервер запускается по адресу:

```text
http://127.0.0.1:8080
```

Пример открытия страницы:

```text
http://127.0.0.1:8080/downtimes/workpiece
```

Для production-сборки:

```bash
cargo build --release
```

Исполняемый файл будет находиться в:

```text
target/release/
```

## Основные маршруты

### Страницы

```http
GET /downtimes/{area}
```

Пример:

```http
GET /downtimes/rolling
```

### Список простоев

```http
GET /api/{area}/downtimes
```

Возвращает последние простои выбранного участка.

### Фильтрация по дате

```http
POST /api/{area}/downtimes/filter
Content-Type: application/json
```

Пример тела запроса:

```json
{
  "Filter_StartDate": "20260724000000",
  "Filter_EndDate": "20260724235959",
  "WithEAMDowntimes": 1,
  "ShowDeleted": 0,
  "MaxRows": 50,
  "Offset": 0
}
```

`Filter_DepartmentID` устанавливается backend на основании `{area}`.

### Поиск оборудования

```http
GET /api/{area}/equipment?q=текст
```

Оборудование фильтруется по `DTEquipments.DepartmentID`.

### Поиск мастеров

```http
GET /api/{area}/workers?q=текст
```

Мастера фильтруются по:

```sql
Place_id = Place ID текущего участка
AND Post_id = 1
```

### Типы простоев

```http
GET /api/downtime-types
```

### Ответственные службы

```http
GET /api/departments
```

### Получение одного простоя

```http
POST /api/get_downtime
Content-Type: application/json
```

```json
{
  "EAMID": "ff4d74a8-83fa-11f1-a492-00155d011a0f"
}
```

### Создание или редактирование

```http
POST /api/update_downtime
Content-Type: application/json
```

## Создание простоя для нескольких оборудований

```json
{
  "Area": "workpiece",
  "EAMID": "",
  "EquipmentIDs": [
    "00ba56a9-be27-11ed-8163-00155d16fe68",
    "02e9d373-be24-11ed-8163-00155d16fe68"
  ],
  "DepartmentID": "92b138d1-c131-11ed-8163-00155d16fe68",
  "DepartmentIDs": [
    "40fe0563-c04a-11f0-a48d-00155d011305"
  ],
  "DowntimeTypeID": "b13c6a2c-58c2-11f1-a490-00155d011305",
  "StartDateTime": "20260724110000",
  "EndDateTime": "",
  "Author": "Жапбар И.Б.",
  "Comment": "Комбинированный отказ оборудования"
}
```

Поля:

- `Area` — slug текущего производственного участка;
- `EquipmentIDs` — оборудование текущего участка;
- `DepartmentID` — `DepartmentID` производственного участка;
- `DepartmentIDs` — выбранные ответственные службы;
- `EAMID` — пустая строка при создании;
- `StartDateTime` и `EndDateTime` — дата в формате `yyyyMMddHHmmss`.

Оборудование разных участков нельзя смешивать в одном запросе.

## Редактирование простоя

При редактировании используется одиночный `EquipmentID` и заполненный `EAMID`:

```json
{
  "Area": "workpiece",
  "EAMID": "ff4d74a8-83fa-11f1-a492-00155d011a0f",
  "EquipmentID": "00ba56a9-be27-11ed-8163-00155d16fe68",
  "DepartmentID": "92b138d1-c131-11ed-8163-00155d16fe68",
  "DowntimeTypeID": "b13c6a2c-58c2-11f1-a490-00155d011305",
  "StartDateTime": "20260724110000",
  "EndDateTime": "20260724120000",
  "Author": "Жапбар И.Б.",
  "Comment": "Изменённые сведения о простое"
}
```

`EAMVersion` загружается из локальной таблицы `Downtimes` перед отправкой изменений в ТОИР.

## Используемые таблицы

### База `Downtime`

#### `dbo.Downtimes`

Локальные данные простоев:

```text
Id
EquipmentID
DepartmentID
DowntimeTypeID
EAMDocName
EAMVersion
Comment
Author
StartDateTime
EndDateTime
```

#### `dbo.DTEquipments`

Справочник оборудования:

```text
Id
Code
Name
DepartmentID
Marked
CreatedAt
ModifiedAt
```

#### `dbo.DTDepartments`

Справочник подразделений:

```text
Id
Code
Name
Marked
CreatedAt
ModifiedAt
```

#### `dbo.DowntimesTypes`

Справочник типов простоев.

### База `WheelTrack`

#### `dbo.Worker`

Используется для поиска мастеров по `Place_id` и `Post_id`.

#### `dbo.Place`

Содержит маршруты производственных участков, в том числе:

```text
Downtime_route
```

Примеры:

```text
/downtimes/workpiece
/downtimes/rolling
/downtimes/utk
/downtimes/kp
/downtimes/unk
/downtimes/uppo
/downtimes/unk
```

## Изменения базы данных

### Поле департамента в `Downtimes`

```sql
IF COL_LENGTH('dbo.Downtimes', 'DepartmentID') IS NULL
BEGIN
    ALTER TABLE dbo.Downtimes
    ADD DepartmentID BINARY(16) NULL;
END;
```

### Внешний ключ

```sql
IF NOT EXISTS (
    SELECT 1
    FROM sys.foreign_keys
    WHERE name = N'FK_Downtimes_DTDepartments'
)
BEGIN
    ALTER TABLE dbo.Downtimes
    ADD CONSTRAINT FK_Downtimes_DTDepartments
        FOREIGN KEY (DepartmentID)
        REFERENCES dbo.DTDepartments(Id);
END;
```

### Маршруты простоев в `Place`

```sql
IF COL_LENGTH('dbo.Place', 'Downtime_route') IS NULL
BEGIN
    ALTER TABLE dbo.Place
    ADD Downtime_route NVARCHAR(250) NULL;
END;

UPDATE dbo.Place
SET Downtime_route =
    CASE ID
        WHEN 1  THEN N'/downtimes/workpiece'
        WHEN 4  THEN N'/downtimes/rolling'
        WHEN 5  THEN N'/downtimes/unk'
        WHEN 7  THEN N'/downtimes/utk'
        WHEN 10 THEN N'/downtimes/uppo'
        ELSE Downtime_route
    END
WHERE ID IN (1, 4, 5, 7, 10);
```

## Особенности работы с ТОИР

- запросы отправляются с Basic Authentication;
- данные передаются в JSON;
- при создании нескольких простоев backend перебирает `EquipmentIDs`;
- каждое оборудование отправляется в ТОИР отдельным запросом;
- локальная запись выполняется только после успешного ответа ТОИР;
- backend дополнительно фильтрует ответ по `DepartmentID` текущего участка.

## Безопасность

- не храните `.env` в Git;
- не выводите логин, пароль и строку подключения в журнал;
- не принимайте произвольный `DepartmentID` от frontend без проверки;
- определяйте участок через `find_area()`;
- перед сохранением проверяйте принадлежность оборудования выбранному участку;
- в production не рекомендуется использовать `danger_accept_invalid_certs(true)`.

## Проверка

Перед публикацией рекомендуется проверить:

```bash
cargo fmt --check
cargo check
cargo test
```

Проверка frontend JavaScript:

```bash
node --check static/js/downtimes.js
```

## Автор

Сектор информационных технологий  
АО «Проммашкомплект»
