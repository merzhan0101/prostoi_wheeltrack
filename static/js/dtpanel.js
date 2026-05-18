let currentEditType = 'equip';

document.addEventListener('DOMContentLoaded', function () {
    // --- 1. ЭЛЕМЕНТЫ ---
    const tableBody = document.getElementById('table-body');
    const tableHead = document.querySelector('thead tr');

    // Модалка редактирования
    const modal = document.getElementById('modalSettings');
    const closeBtn = document.querySelector('.close-modal');
    const modalForm = modal.querySelector('form');

    // Модалка добавления (НОВОЕ)
    const modalAdd = document.getElementById('modalAdd');
    const btnAddNew = document.getElementById('addNewBtn');
    const closeAdd = document.getElementById('closeAdd');
    const addForm = document.getElementById('addForm');
    const selectAddType = document.getElementById('select-add-type');

    // --- 2. ФУНКЦИИ ЗАГРУЗКИ ---
    async function loadData(type) {
        tableBody.innerHTML = '<tr><td colspan="6" style="text-align:center">Загрузка...</td></tr>';

        let url = '';
        if (type === 'equip') url = '/api/equipments';
        if (type === 'depts') url = '/api/departments';
        if (type === 'types') url = '/api/types';

        try {
            const response = await fetch(url);
            const data = await response.json();
            renderTable(type, data);
        } catch (error) {
            console.error("Ошибка:", error);
            tableBody.innerHTML = '<tr><td colspan="6" style="color:red; text-align:center;">Ошибка подключения к серверу</td></tr>';
        }
    }

    // Универсальная функция загрузки отделов в любой селект
async function fillDepartmentSelect(selectElement, targetDeptName = null) {
    try {
        // МЕНЯЕМ ТУТ: запрашиваем только активные для выбора
        const response = await fetch('/api/departments/active'); 
        const depts = await response.json();

        selectElement.innerHTML = '<option value="">Выберите отдел...</option>';

        depts.forEach(dept => {
            const option = document.createElement('option');
            option.value = dept.id;
            option.textContent = dept.name;
            // Если мы редактируем и отдел совпадает, выбираем его
            if (targetDeptName && dept.name === targetDeptName) {
                option.selected = true;
            }
            selectElement.appendChild(option);
        });
    } catch (error) {
        console.error("Ошибка загрузки отделов:", error);
    }
}

    function renderTable(type, data) {
        tableBody.innerHTML = '';

        if (type === 'equip') {
            tableHead.innerHTML = `<th>ID</th><th>Код</th><th>Наименование</th><th>Отдел</th><th>Статус</th><th style="text-align:right">Действия</th>`;
            data.forEach(item => {
                tableBody.innerHTML += `
                <tr>
                    <td>${item.id}</td>
                    <td><span class="duration-badge">${item.code}</span></td>
                    <td>${item.name}</td>
                    <td>${item.dept}</td>
                    <td><span class="status-pill ${item.active ? 'active' : ''}">${item.active ? 'Активен' : 'Выкл'}</span></td>
                    <td style="text-align:right">
                        <button class="action-btn btn-edit" data-id="${item.id}" data-type="equip">
                            <i class="fas fa-pen"></i>
                        </button>
                    </td>
                </tr>`;
            });
        }
        else if (type === 'depts') {
            tableHead.innerHTML = `<th>ID</th><th>Код</th><th>Название отдела</th><th>Статус</th><th style="text-align:right">Действия</th>`;
            data.forEach(item => {
                tableBody.innerHTML += `
                <tr>
                    <td>${item.id}</td>
                    <td>${item.code}</td>
                    <td>${item.name}</td>
                    <td><span class="status-pill ${item.active ? 'active' : ''}">${item.active ? 'Активен' : 'Выкл'}</span></td>
                    <td style="text-align:right">
                        <button class="action-btn btn-edit" data-id="${item.id}" data-type="dept">
                            <i class="fas fa-pen"></i>
                        </button>
                    </td>
                </tr>`;
            });
        }
        else if (type === 'types') {
            tableHead.innerHTML = `<th>ID</th><th>Тип простоя</th><th>Статус</th><th style="text-align:right">Действия</th>`;
            data.forEach(item => {
                tableBody.innerHTML += `
                <tr>
                    <td>${item.id}</td>
                    <td>${item.name}</td>
                    <td><span class="status-pill ${item.active ? 'active' : ''}">${item.active ? 'Активен' : 'Выкл'}</span></td>
                    <td style="text-align:right">
                        <button class="action-btn btn-edit" data-id="${item.id}" data-type="type">
                            <i class="fas fa-pen"></i>
                        </button>
                    </td>
                </tr>`;
            });
        }
    }

    // --- 3. МОДАЛКА РЕДАКТИРОВАНИЯ ---
    function openModal(id, code, name, type, currentDeptName) {
        currentEditType = type;
        const modalIdBadge = modal.querySelector('.modal-badge');
        const inputCode = modal.querySelector('input[placeholder="Напр: CNC-001"]');
        const inputName = modal.querySelector('input[placeholder="Введите название"]');
        const deptGroup = modal.querySelector('.service-select').closest('.input-group');
        const deptSelect = modal.querySelector('.service-select');

        modalIdBadge.innerText = `ID: ${id}`;
        modalIdBadge.dataset.id = id;

        inputCode.closest('.input-group').style.display = (type === 'type') ? 'none' : 'flex';

        if (type === 'equip') {
            deptGroup.style.display = 'flex';
            fillDepartmentSelect(deptSelect, currentDeptName);
        } else {
            deptGroup.style.display = 'none';
        }

        if (inputCode) inputCode.value = code || '';
        if (inputName) inputName.value = name || '';
        document.getElementById('act-yes').checked = true;

        modal.style.display = 'flex';
    }

    // --- 4. ЛОГИКА ДОБАВЛЕНИЯ (НОВОЕ) ---
    // --- 4. ЛОГИКА ДОБАВЛЕНИЯ ---
    function updateAddFormFields() {
        const type = selectAddType.value;

        // Элементы групп (контейнеры)
        const groupCode = document.getElementById('add-group-code');
        const groupName = document.getElementById('add-group-name');
        const groupDept = document.getElementById('add-group-dept');

        // Текстовые метки (label)
        const labelCode = document.getElementById('label-add-code');
        const labelName = document.getElementById('label-add-name');

        const deptSelect = document.getElementById('add-select-dept');

        // Сбрасываем видимость (всё прячем, потом покажем нужное)
        if (groupCode) groupCode.style.display = 'none';
        if (groupDept) groupDept.style.display = 'none';
        if (groupName) groupName.style.display = 'flex';

        if (type === 'equip') {
            // Оборудование: Код + Наименование + Отдел
            if (groupCode) groupCode.style.display = 'flex';
            if (groupDept) groupDept.style.display = 'flex';
            if (labelCode) labelCode.innerText = 'Код оборудования';
            if (labelName) labelName.innerText = 'Наименование оборудования';
            fillDepartmentSelect(deptSelect);
        }
        else if (type === 'dept') {
            // Отделы: Код + Название отдела
            if (groupCode) groupCode.style.display = 'flex';
            if (labelCode) labelCode.innerText = 'Код отдела';
            if (labelName) labelName.innerText = 'Название отдела';
        }
        else if (type === 'type') {
            // Типы простоев: только Название
            if (labelName) labelName.innerText = 'Тип простоя';
        }
    }

    // Слушатель на кнопку "Добавить запись"
    if (btnAddNew) {
        btnAddNew.addEventListener('click', () => {
            if (addForm) addForm.reset();
            updateAddFormFields(); // Теперь не упадет
            if (modalAdd) modalAdd.style.display = 'flex';
        });
    }

    btnAddNew.addEventListener('click', () => {
        addForm.reset();
        updateAddFormFields();
        modalAdd.style.display = 'flex';
    });

    closeAdd.addEventListener('click', () => modalAdd.style.display = 'none');
    selectAddType.addEventListener('change', updateAddFormFields);

    addForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        // Получаем значение статуса из радиокнопок
        const isActive = document.getElementById('add-act-yes').checked;

        const payload = {
            type: selectAddType.value,
            code: document.getElementById('add-input-code').value,
            name: document.getElementById('add-input-name').value,
            dept_id: document.getElementById('add-select-dept').value || null,
            active: isActive // Теперь берем реальное значение из формы
        }; 

        try {
            const response = await fetch('/api/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            if (response.ok) {
                modalAdd.style.display = 'none';
                // Перезагружаем таблицу, на которой сейчас находимся
                loadData(payload.type === 'equip' ? 'equip' : (payload.type === 'dept' ? 'depts' : 'types'));
            } else {
                alert("Ошибка при создании");
            }
        } catch (err) {
            console.error("Ошибка сети:", err);
        }
    });

    // --- 5. СЛУШАТЕЛИ СОБЫТИЙ ---
    document.getElementById('tab-equip').addEventListener('change', () => loadData('equip'));
    document.getElementById('tab-depts').addEventListener('change', () => loadData('depts'));
    document.getElementById('tab-types').addEventListener('change', () => loadData('types'));

    closeBtn.addEventListener('click', () => modal.style.display = 'none');
    window.addEventListener('click', (e) => {
        if (e.target === modal) modal.style.display = 'none';
        if (e.target === modalAdd) modalAdd.style.display = 'none';
    });

    tableBody.addEventListener('click', function (e) {
        const editBtn = e.target.closest('.btn-edit');
        if (editBtn) {
            const row = editBtn.closest('tr');
            const id = editBtn.dataset.id;
            const type = editBtn.dataset.type;
            let code = '', name = '', currentDeptName = '';

            if (type === 'equip') {
                code = row.cells[1].innerText;
                name = row.cells[2].innerText;
                currentDeptName = row.cells[3].innerText;
            } else if (type === 'dept') {
                code = row.cells[1].innerText;
                name = row.cells[2].innerText;
            } else {
                name = row.cells[1].innerText;
            }
            openModal(id, code, name, type, currentDeptName);
        }
    });

    if (modalForm) {
        modalForm.addEventListener('submit', async function (e) {
            e.preventDefault();
            const id = modal.querySelector('.modal-badge').dataset.id;
            const codeVal = modal.querySelector('input[placeholder="Напр: CNC-001"]').value;
            const nameVal = modal.querySelector('input[placeholder="Введите название"]').value;
            const activeVal = document.getElementById('act-yes').checked;
            const deptSelect = modal.querySelector('.service-select');

            const payload = {
                id: id,
                type: currentEditType,
                code: codeVal,
                name: nameVal,
                active: activeVal,
                dept_id: (currentEditType === 'equip') ? deptSelect.value : null
            };

            try {
                const response = await fetch('/api/update', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                if (response.ok) {
                    modal.style.display = 'none';
                    loadData(currentEditType === 'equip' ? 'equip' : (currentEditType === 'dept' ? 'depts' : 'types'));
                } else { alert("Ошибка при сохранении"); }
            } catch (err) { console.error(err); }
        });
    }

    // --- СТАРТ ---
    loadData('equip');
});