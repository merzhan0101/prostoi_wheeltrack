function getCurrentAreaSlug() {
  const element =
    document.getElementById('current-area-slug');

  if (!element || !element.value) {
    throw new Error(
      'Не удалось определить текущий участок'
    );
  }

  return element.value;
}

function getCurrentAreaDepartmentId() {
  const element = document.getElementById(
    'current-area-department-id'
  );

  if (!element || !element.value) {
    throw new Error(
      'Не удалось определить DepartmentID участка'
    );
  }

  return element.value;
}

    // for alert
    function showToast(message) {
        const toast = document.getElementById('custom-toast');
        const text = document.getElementById('toast-message');

        text.textContent = message;

        toast.classList.add('show');

        setTimeout(() => {
            toast.classList.remove('show');
        }, 3000);
    }

document.addEventListener('DOMContentLoaded', function () {
    const currentAreaSlugElement =
        document.getElementById('current-area-slug');

    const currentAreaDepartmentElement =
        document.getElementById(
        'current-area-department-id'
        );

    if (
        !currentAreaSlugElement ||
        !currentAreaDepartmentElement
    ) {
        console.error(
        'Не найдены данные текущего участка'
        );

        return;
    }

    // const CURRENT_AREA_SLUG =
    //     currentAreaSlugElement.value;

    // const CURRENT_AREA_DEPARTMENT_ID =
    //     currentAreaDepartmentElement.value;

    // 'DOMContentLoaded',
    // function () {
    //     console.log('Текущий участок:', {
    //     slug: getCurrentAreaSlug(),
    //     departmentId: getCurrentAreaDepartmentId()
    //     });
    // }
    
    // --- ЭЛЕМЕНТЫ МОДАЛКИ ДОБАВЛЕНИЯ ---
    const entryModal = document.getElementById('entryModal');
    const addNewBtn = document.getElementById('addNewBtn');
    const closeModal = document.querySelector('.close-modal');
    const entryForm = document.getElementById('entryForm');

    // Элементы выбора оборудования
    const eqSearchInput = document.getElementById('modal-equipment-search');
    const eqDropdown = document.getElementById('eq-dropdown');
    const eqHiddenId = document.getElementById('modal-equipment-id');



    let selectedEquipments = [];

    function renderSelectedEquipments() {
        const box = document.getElementById('selected-equipments');
        if (!box) return;

        box.innerHTML = selectedEquipments.map(eq => `
            <span class="equipment-chip">
                ${eq.name}
                <button type="button" data-id="${eq.id}">×</button>
            </span>
        `).join('');

        box.querySelectorAll('button').forEach(btn => {
            btn.onclick = () => {
                selectedEquipments = selectedEquipments.filter(x => x.id !== btn.dataset.id);
                renderSelectedEquipments();
            };
        });
    }

    let selectedResponsibleDepartments = [];

    function renderSelectedDepartments() {
        const box = document.getElementById('selected-departments');
        if (!box) return;

        box.innerHTML = selectedResponsibleDepartments.map(dep => `
            <span class="equipment-chip">
                ${dep.name}
                <button type="button" data-id="${dep.id}">×</button>
            </span>
        `).join('');

        box.querySelectorAll('button').forEach(btn => {
            btn.onclick = () => {
                selectedResponsibleDepartments = selectedResponsibleDepartments.filter(x => x.id !== btn.dataset.id);
                renderSelectedDepartments();
            };
        });
    }

    // ПОИСК СЛУЖБ
    const departmentSearchInput = document.getElementById('modal-department-search');
    const departmentDropdown = document.getElementById('department-dropdown');

    const fetchDepartments = async (query) => {
        try {
            const response = await fetch(`/api/departments?q=${encodeURIComponent(query)}`);
            const data = await response.json();

            if (data.length > 0) {
                departmentDropdown.innerHTML = data.map(item => `
                    <div class="dropdown-item" data-id="${item.id}">
                        ${item.department}
                    </div>
                `).join('');
            } else {
                departmentDropdown.innerHTML = '<div class="dropdown-item">Служба не найдена</div>';
            }

            departmentDropdown.style.display = 'block';
        } catch (e) {
            console.error("Ошибка поиска служб:", e);
        }
    };

    if (departmentSearchInput) {
        departmentSearchInput.onfocus = () => fetchDepartments(departmentSearchInput.value);
        departmentSearchInput.oninput = (e) => fetchDepartments(e.target.value);
    }

    if (departmentDropdown) {
        departmentDropdown.onclick = (e) => {
            const item = e.target.closest('.dropdown-item');

            if (item && item.dataset.id) {
                const exists = selectedResponsibleDepartments.some(x => x.id === item.dataset.id);

                if (!exists) {
                    selectedResponsibleDepartments.push({
                        id: item.dataset.id,
                        name: item.textContent.trim()
                    });
                }

                departmentSearchInput.value = "";
                renderSelectedDepartments();

                departmentDropdown.style.display = "none"; // закрыть
            }
        };
    }

    document.addEventListener('click', (e) => {
        const container = document.getElementById('department-container');
        if (container && !container.contains(e.target)) {
            departmentDropdown.style.display = 'none';
        }
    });

    // Элемент выбора типа простоя
    const typeSelect = document.getElementById('modal-type');

    const workerSearchInput = document.getElementById('modal-worker-search');
    const workerDropdown = document.getElementById('worker-dropdown');
    const workerHiddenId = document.getElementById('modal-worker-id');

    const fetchWorkers = async (query) => {
        try {
            // const response = await fetch(`/api/workers?q=${encodeURIComponent(query)}`);

            const areaSlug = getCurrentAreaSlug();
            const response = await fetch(
                `/api/${areaSlug}/workers` +
                `?q=${encodeURIComponent(query)}`
            );

            const data = await response.json();

            if (data.length > 0) {
                workerDropdown.innerHTML = data.map(item => `
                    <div class="dropdown-item" data-id="${item.id}">
                        ${item.fio}
                    </div>
                `).join('');
                workerDropdown.style.display = 'block';
            } else {
                workerDropdown.innerHTML = '<div class="dropdown-item">Сотрудник не найден</div>';
                workerDropdown.style.display = 'block';
            }
        } catch (e) { console.error("Ошибка поиска сотрудников:", e); }
    };

    if (workerSearchInput) {
        workerSearchInput.onfocus = () => fetchWorkers(workerSearchInput.value);
        workerSearchInput.oninput = (e) => fetchWorkers(e.target.value);
    }

    // Выбор сотрудника из списка
    if (workerDropdown) {
        workerDropdown.onclick = (e) => {
            const item = e.target.closest('.dropdown-item');
            if (item && item.dataset.id) {
                workerSearchInput.value = item.textContent.trim();
                workerHiddenId.value = item.dataset.id; // сохраняем UUID сотрудника
                workerDropdown.style.display = 'none';
            }
        };
    }

    // Дополнительно: закрой выпадашку сотрудников при клике вне поля
    document.addEventListener('click', (e) => {
        const container = document.getElementById('worker-container');
        if (container && !container.contains(e.target)) {
            workerDropdown.style.display = 'none';
        }
    });


    // --- 1. ЛОГИКА МОДАЛКИ (ОТКРЫТИЕ/ЗАКРЫТИЕ) ---
    if (addNewBtn) {
        addNewBtn.onclick = () => {
            entryModal.style.display = 'flex';
            loadDowntimeTypes();
            loadDepartments();
        };
    }

    if (closeModal) {
        closeModal.onclick = () => {
            entryModal.style.display = 'none';
            entryForm.reset();
            eqHiddenId.value = '';
        };
    }

    // Закрытие по клику вне модалки
    window.onclick = (event) => {
        if (event.target == entryModal) {
            entryModal.style.display = 'none';
            entryForm.reset();
        }
    };

    // --- 2. ТИПЫ ПРОСТОЕВ (SELECT) ---
    // ВАЖНО: Объявите её ВНЕ DOMContentLoaded или уберите const/let, чтобы она стала глобальной
    async function loadDowntimeTypes(selectId = 'modal-type', selectedValue = null) {
        const typeSelect = document.getElementById(selectId);
        if (!typeSelect) return;

        // Если в списке уже есть данные (кроме дефолтной фразы), не грузим заново
        if (typeSelect.options.length > 1 && !selectedValue) return;

        try {
            const response = await fetch('/api/downtime-types');
            const data = await response.json();

            // Очищаем текущие опции (кроме первой "Выберите тип...")
            typeSelect.innerHTML = '<option value="">Выберите тип...</option>';

            data.forEach(t => {
                let opt = new Option(t.name, t.id);
                if (selectedValue && t.id === selectedValue) {
                    opt.selected = true;
                }
                typeSelect.add(opt);
            });
        } catch (e) {
            console.error("Ошибка загрузки типов:", e);
        }
    }

    // Чтобы функция была доступна везде
    window.loadDowntimeTypes = loadDowntimeTypes;

    // СЛУЖБЫ
    async function loadDepartments(selectId = 'modal-department', selectedValue = null) {
        const typeSelect = document.getElementById(selectId);
        if (!typeSelect) return;

        // Если в списке уже есть данные (кроме дефолтной фразы), не грузим заново
        if (typeSelect.options.length > 1 && !selectedValue) return;

        try {
            const response = await fetch('/api/departments');
            const data = await response.json();
            // console.log("Загруженные службы:", data);

            // Очищаем текущие опции (кроме первой "Выберите службу...")
            typeSelect.innerHTML = '<option value="">Выберите службу...</option>';

            data.forEach(t => {
                let opt = new Option(t.department, t.id);
                if (selectedValue && t.id === selectedValue) {
                    opt.selected = true;
                }
                typeSelect.add(opt);
            });
        } catch (e) {
            console.error("Ошибка загрузки служб:", e);
        }
    }

    // Чтобы функция была доступна везде
    window.loadDepartments = loadDepartments;

    // --- 3. ОБОРУДОВАНИЕ (ЖИВОЙ ПОИСК) ---
    const fetchEquipment = async (query) => {
        try {
            const areaSlug = getCurrentAreaSlug();

            const response = await fetch(
                `/api/${areaSlug}/equipment` +
                `?q=${encodeURIComponent(query)}`
            );
            const data = await response.json();

            if (data.length > 0) {
                eqDropdown.innerHTML = data.map(item => `
                    <div class="dropdown-item" data-id="${item.id}">
                        ${item.name}
                    </div>
                `).join('');
                eqDropdown.style.display = 'block';
            } else {
                eqDropdown.innerHTML = '<div class="dropdown-item">Ничего не найдено</div>';
                eqDropdown.style.display = 'block';
            }
        } catch (e) { console.error("Ошибка поиска оборудования:", e); }
    };

    if (eqSearchInput) {
        eqSearchInput.onfocus = () => fetchEquipment(eqSearchInput.value);
        eqSearchInput.oninput = (e) => fetchEquipment(e.target.value);
    }

    // Выбор из списка оборудования
    if (eqDropdown) {
        eqDropdown.onclick = (e) => {
            const item = e.target.closest('.dropdown-item');

            if (item && item.dataset.id) {

                /*const typeText = document
                    .getElementById('modal-type')
                    .selectedOptions[0]
                    ?.textContent
                    ?.trim();

                const combinedTypes = [
                    'Комбинированный отказ',
                    'Ожидание смежного участка',
                    'Калибровка'
                ];

                const isCombined = combinedTypes.includes(typeText);

                if (isCombined) {
                    const exists = selectedEquipments.some(x => x.id === item.dataset.id);

                    if (!exists) {
                        selectedEquipments.push({
                            id: item.dataset.id,
                            name: item.textContent.trim()
                        });
                    }

                    eqSearchInput.value = '';
                    eqHiddenId.value = '';
                    renderSelectedEquipments();
                } else {
                    selectedEquipments = [{
                        id: item.dataset.id,
                        name: item.textContent.trim()
                    }];

                    eqSearchInput.value = item.textContent.trim();
                    eqHiddenId.value = item.dataset.id;
                    renderSelectedEquipments();
                }*/

                const exists = selectedEquipments.some(
                    x => x.id === item.dataset.id
                );

                if (!exists) {
                    selectedEquipments.push({
                        id: item.dataset.id,
                        name: item.textContent.trim()
                    });
                }

                eqSearchInput.value = '';
                eqHiddenId.value = '';

renderSelectedEquipments();

                eqDropdown.style.display = 'none';
            }
        };
    }

    // Закрытие списка при клике вне контейнера
    document.addEventListener('click', (e) => {
        const container = document.getElementById('eq-container');
        if (container && !container.contains(e.target)) {
            eqDropdown.style.display = 'none';
        }
    });

    // --- 4. АВТОРАСЧЕТ ДЛИТЕЛЬНОСТИ ---
    const startInput = document.getElementById('modal-start');
    const endInput = document.getElementById('modal-end');
    const durationInput = document.getElementById('modal-duration'); // Убедись, что этот ID есть в HTML

    const calcDuration = () => {
        if (startInput && endInput && durationInput && startInput.value && endInput.value) {
            const diff = new Date(endInput.value) - new Date(startInput.value);
            if (diff > 0) {
                const hours = Math.floor(diff / 3600000);
                const mins = Math.floor((diff % 3600000) / 60000);
                durationInput.value = `${hours}ч ${mins}м`;
            } else {
                durationInput.value = "Ошибка даты";
            }
        }
    };
    if (startInput) startInput.onchange = calcDuration;
    if (endInput) endInput.onchange = calcDuration;

    // Находим форму создания (entryForm)
    if (entryForm) {
        entryForm.onsubmit = async (e) => {
            e.preventDefault();

            // Берем ID оборудования именно из скрытого поля!
            /*const typeText = document
                .getElementById('modal-type')
                .selectedOptions[0]
                ?.textContent
                ?.trim();

            const combinedTypes = [
                'Комбинированный отказ',
                'Ожидание смежного участка',
                'Калибровка'
            ];

            const isCombined = combinedTypes.includes(typeText);

            const equipmentIds = isCombined
                ? selectedEquipments.map(x => x.id)
                : [document.getElementById('modal-equipment-id').value];*/

            const equipmentIds = selectedEquipments.map(x => x.id);

            const typeId = document.getElementById('modal-type').value;
            const comment = document.getElementById('modal-comment').value.trim();

            if (equipmentIds.length === 0) {
                alert("Ошибка: Выберите хотя бы одно оборудование!");
                return;
            }

            // if (equipmentIds.length === 0 || equipmentIds.some(x => !x)) {
            //     alert("Ошибка: Выберите оборудование из выпадающего списка!");
            //     return;
            // }

            if (!comment) {
                alert("Ошибка: Поле 'Комментарий' обязательно для заполнения!");
                document.getElementById('modal-comment').style.borderColor = 'red';
                document.getElementById('modal-comment').focus();
                return;
            }

            document.getElementById('modal-comment').addEventListener('input', function() {
                this.style.borderColor = '';
            });

            /*if (selectedResponsibleDepartments.length === 0) {
                alert("Ошибка: выберите хотя бы одну службу!");
                return;
            }*/

            const payload = {
                // Area: CURRENT_AREA_SLUG,
                Area: getCurrentAreaSlug(),
                DepartmentID: getCurrentAreaDepartmentId(),

                EAMID: "",
                EquipmentIDs: equipmentIds,
                DepartmentIDs: selectedResponsibleDepartments.map(
                    department => department.id
                ),
                DowntimeTypeID: typeId,
                StartDateTime: formatToEAMDate(
                    document.getElementById('modal-start').value
                ),
                EndDateTime: formatToEAMDate(
                    document.getElementById('modal-end').value
                ),
                Author: document.getElementById(
                    'modal-worker-search'
                ).value,
                Comment: comment
            };

            try {
                const response = await fetch('/api/update_downtime', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                const result = await response.json();

                if (response.ok && result.Posted === 1) {
                    showToast("Простой успешно сохранён");

                    setTimeout(() => {
                        location.reload();
                    }, 1200);
                } else {
                    alert("Ошибка сохранения: " + (result.ErrorMessage || "Неизвестная ошибка"));
                }
            } catch (error) {
                alert("Сетевая ошибка: сервер не отвечает");
            }
        };
    }

    // --- 6. ФИЛЬТРАЦИЯ ПО ДАТЕ (ИСПРАВЛЕНО) ---
    const filterDateInput = document.getElementById('filter-date-start');
    const filterBtn = document.querySelector('.btn-filter');

    if (filterBtn && filterDateInput) {
        filterBtn.onclick = () => {
            loadMainTableData(filterDateInput.value);
        };
    }

});
// Находим элементы для редактирования
const editSearchInput = document.getElementById('edit-equipment-search');
const editDropdown = document.getElementById('edit-eq-dropdown');
const editHiddenInput = document.getElementById('edit-equipment-id');

// Вешаем ту же самую логику, что и для создания
if (editSearchInput) {
    editSearchInput.addEventListener('input', function () {
        const query = this.value;
        if (query.length < 2) {
            editDropdown.style.display = 'none';
            return;
        }

        // Пользуемся твоим готовым роутом /api/equipment
        const areaSlug = getCurrentAreaSlug();
            fetch(
            `/api/${areaSlug}/equipment` +
            `?q=${encodeURIComponent(query)}`
            )
            .then(res => res.json())
            .then(items => {
                editDropdown.innerHTML = '';
                items.forEach(item => {
                    const div = document.createElement('div');
                    div.className = 'dropdown-item';
                    div.textContent = item.name;
                    div.onclick = () => {
                        editSearchInput.value = item.name;
                        editHiddenInput.value = item.id;
                        editDropdown.style.display = 'none';
                    };
                    editDropdown.appendChild(div);
                });
                editDropdown.style.display = 'block';
            });
    });
}
// --- УНИВЕРСАЛЬНАЯ ЛОГИКА ПОИСКА (ОБОРУДОВАНИЕ) ---
const setupEquipmentSearch = (inputId, dropdownId, hiddenId, containerId) => {
    const input = document.getElementById(inputId);
    const dropdown = document.getElementById(dropdownId);
    const hidden = document.getElementById(hiddenId);
    const container = document.getElementById(containerId);

    if (!input || !dropdown) return;

    const fetchEq = async (query) => {
        try {
            const areaSlug = getCurrentAreaSlug();

            const response = await fetch(
                `/api/${areaSlug}/equipment` +
                `?q=${encodeURIComponent(query)}`
            );
            const data = await response.json();
            dropdown.innerHTML = data.map(item => `
                <div class="dropdown-item" data-id="${item.id}">${item.name}</div>
            `).join('') || '<div class="dropdown-item">Ничего не найдено</div>';
            dropdown.style.display = 'block';
        } catch (e) { console.error("Ошибка оборудования:", e); }
    };

    input.onfocus = () => fetchEq(input.value);
    input.oninput = (e) => fetchEq(e.target.value);

    dropdown.onclick = (e) => {
        const item = e.target.closest('.dropdown-item');
        if (item && item.dataset.id) {
            input.value = item.textContent.trim();
            hidden.value = item.dataset.id;
            dropdown.style.display = 'none';
        }
    };

    document.addEventListener('click', (e) => {
        if (container && !container.contains(e.target)) dropdown.style.display = 'none';
    });
};

// --- УНИВЕРСАЛЬНАЯ ЛОГИКА ПОИСКА (РАБОТНИКИ) ---
const setupWorkerSearch = (inputId, dropdownId, hiddenId, containerId) => {
    const input = document.getElementById(inputId);
    const dropdown = document.getElementById(dropdownId);
    const hidden = document.getElementById(hiddenId);
    const container = document.getElementById(containerId);

    if (!input || !dropdown) return;

    const fetchW = async (query) => {
        try {
            // const response = await fetch(`/api/workers?q=${encodeURIComponent(query)}`);
            const areaSlug = getCurrentAreaSlug();

            const response = await fetch(
                `/api/${areaSlug}/workers` +
                `?q=${encodeURIComponent(query)}`
            );

            const data = await response.json();
            dropdown.innerHTML = data.map(item => `
                <div class="dropdown-item" data-id="${item.id}">${item.fio}</div>
            `).join('') || '<div class="dropdown-item">Сотрудник не найден</div>';
            dropdown.style.display = 'block';
        } catch (e) { console.error("Ошибка работников:", e); }
    };

    input.onfocus = () => fetchW(input.value);
    input.oninput = (e) => fetchW(e.target.value);

    dropdown.onclick = (e) => {
        const item = e.target.closest('.dropdown-item');
        if (item && item.dataset.id) {
            input.value = item.textContent.trim();
            if (hidden) hidden.value = item.dataset.id;
            dropdown.style.display = 'none';
        }
    };

    document.addEventListener('click', (e) => {
        if (container && !container.contains(e.target)) dropdown.style.display = 'none';
    });
};

// --- ЗАПУСК ДЛЯ ОБЕИХ МОДАЛОК ---
// Для новой записи
// setupEquipmentSearch('modal-equipment-search', 'eq-dropdown', 'modal-equipment-id', 'eq-container');
setupWorkerSearch('modal-worker-search', 'worker-dropdown', 'modal-worker-id', 'worker-container');

// Для редактирования (добавьте эти ID в HTML если их нет)
setupEquipmentSearch('edit-equipment-search', 'edit-eq-dropdown', 'edit-equipment-id', 'edit-eq-container');
setupWorkerSearch('edit-worker-search', 'edit-worker-dropdown', 'edit-worker-id', 'edit-worker-container');

// Перемещаем функции, которые вызываются из HTML (onclick), в глобальную видимость
// window.openEditModal = async function (id) {
window.openEditModal = async function (id, tableComment = "") {
    const modal = document.getElementById('editModal');
    const displayId = document.getElementById('edit-id-display');
    const entryIdInput = document.getElementById('edit-entry-id');

    if (displayId) displayId.innerText = id.substring(0, 8);
    if (entryIdInput) entryIdInput.value = id;

    try {
        const response = await fetch('/api/get_downtime', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ EAMID: id })
        });

        const data = await response.json();
        // console.log("EDIT DATA:", data);

        if (response.ok) {
            // console.log(data)
            document.getElementById('edit-equipment-id').value = data.EquipmentID;
            document.getElementById('edit-equipment-search').value = data.EquipmentName;
            document.getElementById('edit-start').value = formatToDateTimeLocal(data.StartDateTime);
            document.getElementById('edit-end').value = formatToDateTimeLocal(data.EndDateTime);
            document.getElementById('edit-worker-search').value = data.Author;

            /*const commentArea = document.getElementById('edit-comment');
            if (commentArea) {
                commentArea.value = data.Comment || "";
            }

            // Если есть поле комментария в данных
            const commField = document.getElementById('edit-comment');
            if (commField) commField.value = data.Comment || "";*/

            const commField = document.getElementById('edit-comment');
            if (commField) {
                commField.value = data.Comment || tableComment || "";
            }

            await window.loadDowntimeTypes('edit-type', data.DowntimeTypeID);

            modal.style.display = 'flex'; // Используем flex для центрирования
        } else {
            alert('Ошибка при загрузке данных: ' + data.ErrorMessage);
        }
    } catch (err) {
        console.error('Ошибка:', err);
    }
};

window.closeEditModal = function () {
    document.getElementById('editModal').style.display = 'none';
};

// Вспомогательные функции даты
function formatToDateTimeLocal(eamDate) {
    if (!eamDate || eamDate.length < 12) return "";
    const y = eamDate.substring(0, 4);
    const m = eamDate.substring(4, 6);
    const d = eamDate.substring(6, 8);
    const h = eamDate.substring(8, 10);
    const min = eamDate.substring(10, 12);
    return `${y}-${m}-${d}T${h}:${min}`;
}

function formatToEAMDate(htmlDate) {
    if (!htmlDate) return "";
    return htmlDate.replace(/[-T:]/g, "") + "00";
}

document.addEventListener('DOMContentLoaded', function () {
    // --- УНИВЕРСАЛЬНЫЙ ПОИСК ---
    const setupSearch = (inputId, dropdownId, hiddenId, containerId, apiPath, fieldName) => {
        const areaSlug = getCurrentAreaSlug();

        const input = document.getElementById(inputId);
        const dropdown = document.getElementById(dropdownId);
        const hidden = document.getElementById(hiddenId);
        const container = document.getElementById(containerId);

        if (!input || !dropdown) return;

        const fetchData = async (query) => {
            try {
                const response = await fetch(`${apiPath}?q=${encodeURIComponent(query)}`);
                let data = await response.json(); // Используем let, чтобы можно было перезаписать отсортированный массив
                // console.log(data);
                // --- СОРТИРОВКА ПО АЛФАВИТУ ---
                data.sort((a, b) => a[fieldName].localeCompare(b[fieldName]));

                dropdown.innerHTML = data.map(item => `
            <div class="dropdown-item" data-id="${item.id}">${item[fieldName]}</div>
        `).join('') || '<div class="dropdown-item">Ничего не найдено</div>';
                dropdown.style.display = 'block';
            } catch (e) {
                console.error("Ошибка поиска:", e);
            }
        };

        input.onfocus = () => fetchData(input.value);
        input.oninput = (e) => fetchData(e.target.value);

        dropdown.onclick = (e) => {
            const item = e.target.closest('.dropdown-item');
            if (item && item.dataset.id) {
                input.value = item.textContent.trim();
                if (hidden) hidden.value = item.dataset.id;
                dropdown.style.display = 'none';
            }
        };

        document.addEventListener('click', (e) => {
            if (container && !container.contains(e.target)) dropdown.style.display = 'none';
        });
    };

    // Инициализация всех поисков
    // setupSearch('modal-equipment-search', 'eq-dropdown', 'modal-equipment-id', 'eq-container', '/api/equipment', 'name');

    /*setupSearch('edit-equipment-search', 'edit-eq-dropdown', 'edit-equipment-id', 'edit-eq-container', '/api/equipment', 'name');
    setupSearch('modal-worker-search', 'worker-dropdown', 'modal-worker-id', 'worker-container', '/api/workers', 'fio');
    setupSearch('edit-worker-search', 'edit-worker-dropdown', 'edit-worker-id', 'edit-worker-container', '/api/workers', 'fio');*/

    const areaSlug = getCurrentAreaSlug();

    const equipmentApiPath =
      `/api/${areaSlug}/equipment`;

    const workersApiPath =
      `/api/${areaSlug}/workers`;

    setupSearch(
      'edit-equipment-search',
      'edit-eq-dropdown',
      'edit-equipment-id',
      'edit-eq-container',
      equipmentApiPath,
      'name'
    );

    setupSearch(
      'modal-worker-search',
      'worker-dropdown',
      'modal-worker-id',
      'worker-container',
      workersApiPath,
      'fio'
    );

    setupSearch(
      'edit-worker-search',
      'edit-worker-dropdown',
      'edit-worker-id',
      'edit-worker-container',
      workersApiPath,
      'fio'
    );

    // --- МОДАЛКИ ---
    const entryModal = document.getElementById('entryModal');
    const editModal = document.getElementById('editModal');

    document.getElementById('addNewBtn').onclick = () => {
        entryModal.style.display = 'flex';
        window.loadDowntimeTypes('modal-type');
        window.loadDepartments('modal-department');
    };

    document.querySelectorAll('.close-modal').forEach(btn => {
        btn.onclick = () => {
            entryModal.style.display = 'none';
            editModal.style.display = 'none';
        };
    });

    window.onclick = (event) => {
        if (event.target == entryModal) entryModal.style.display = 'none';
        if (event.target == editModal) editModal.style.display = 'none';
    };

    // --- ФУНКЦИЯ ЗАГРУЗКИ ТИПОВ ---
    window.loadDowntimeTypes = async function (selectId, selectedValue = null) {
        const typeSelect = document.getElementById(selectId);
        if (!typeSelect) return;
        try {
            const response = await fetch('/api/downtime-types');
            const data = await response.json();
            typeSelect.innerHTML = '<option value="">Выберите тип...</option>';
            data.forEach(t => {
                let opt = new Option(t.name, t.id);
                if (selectedValue && t.id === selectedValue) opt.selected = true;
                typeSelect.add(opt);
            });
        } catch (e) { console.error("Ошибка типов:", e); }
    };

    // --- ОБРАБОТКА ФОРМ ---
    document.getElementById('editForm').onsubmit = async (e) => {
    e.preventDefault();
    
    const payload = {
        Area: getCurrentAreaSlug(),
        DepartmentID: getCurrentAreaDepartmentId(),
        DepartmentIDs: [getCurrentAreaDepartmentId()],
        EAMID: document.getElementById('edit-entry-id').value,
        EquipmentID: document.getElementById('edit-equipment-id').value,
        StartDateTime: formatToEAMDate(document.getElementById('edit-start').value),
        EndDateTime: formatToEAMDate(document.getElementById('edit-end').value),
        DowntimeTypeID: document.getElementById('edit-type').value,
        Author: document.getElementById('edit-worker-search').value,
        Comment: document.getElementById('edit-comment').value
    };

    try {
        const response = await fetch('/api/update_downtime', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        
        // Получаем ответ как текст сначала для диагностики
        const responseText = await response.text();
        // console.log('Ответ от сервера:', responseText);
        
        // Парсим JSON
        let result;
        try {
            result = JSON.parse(responseText);
        } catch (parseError) {
            console.error('Ошибка парсинга JSON:', parseError);
            alert('❌ Ошибка: сервер вернул некорректный ответ');
            return;
        }
        
        // Проверяем результат
        if (result.Posted === 1) {
            showToast("✅ Простой успешно обновлён");
            setTimeout(() => location.reload(), 1200);
        } else {
            // Показываем понятную ошибку
            const errorMsg = result.ErrorMessage || "Неизвестная ошибка";
            alert("❌ Ошибка: " + errorMsg);
        }
    } catch (err) {
        console.error('Ошибка:', err);
        alert('❌ Сетевая ошибка при сохранении. Проверьте подключение к серверу.');
    }
};

    /*document.getElementById('editForm').onsubmit = async (e) => {
        e.preventDefault();
        const payload = {
            EAMID: document.getElementById('edit-entry-id').value,
            EquipmentID: document.getElementById('edit-equipment-id').value,
            StartDateTime: formatToEAMDate(document.getElementById('edit-start').value),
            EndDateTime: formatToEAMDate(document.getElementById('edit-end').value),
            DowntimeTypeID: document.getElementById('edit-type').value,
            Author: document.getElementById('edit-worker-search').value,
            Comment: document.getElementById('edit-comment').value
        };

        try {
            // console.log(payload);
            const response = await fetch('/api/update_downtime', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const result = await response.json();
            if (result.Posted === 1) {
               showToast("Простой успешно обновлён");

                setTimeout(() => {
                    location.reload();
                }, 1200);
            } else {
                alert('Ошибка: ' + result.ErrorMessage);
            }
        } catch (err) { alert('Ошибка сохранения'); }
    };*/
});

// --- ЗАГРУЗКА ДАННЫХ В ТАБЛИЦУ ---
async function loadMainTableData(dateVal = null) {
    const tableBody =
        document.getElementById('table-body');

    if (!tableBody) {
        return;
    }

    let areaSlug;

    try {
        areaSlug = getCurrentAreaSlug();
    } catch (error) {
        console.error(error);

        tableBody.innerHTML = `
        <tr>
            <td colspan="10" style="color: red; text-align: center;">
            ${error.message}
            </td>
        </tr>
        `;

        return;
    }

    tableBody.innerHTML = `
        <tr>
        <td colspan="10" style="text-align: center;">
            Загрузка данных...
        </td>
        </tr>
    `;

    try {
        let response;

        if (dateVal) {
        const dateForToir =
            dateVal.replaceAll('-', '');

        const payload = {
            Filter_StartDate:
            dateForToir + '000000',

            Filter_EndDate:
            dateForToir + '235959',

            WithEAMDowntimes: 1,
            ShowDeleted: 0,
            MaxRows: 50,
            Offset: 0
        };

        response = await fetch(
            `/api/${areaSlug}/downtimes/filter`,
            {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(payload)
            }
        );
        } else {
        response = await fetch(
            `/api/${areaSlug}/downtimes`
        );
        }

        if (!response.ok) {
        const errorText = await response.text();

        throw new Error(
            `Ошибка API ${response.status}: ${errorText}`
        );
        }

        const data = await response.json();
        // console.log(data); //все данные по простоям

        let items = Array.isArray(data) ? data : Object.values(data);
        // console.log(items); //массив простоев
        const count = items.length;

        if (count === 0) {
            tableBody.innerHTML = '<tr><td colspan="8" style="text-align: center;">Нет данных</td></tr>';
            return;
        }

        const formatDisplayDate = (dateStr) => {
            if (!dateStr) return "-";
            if (dateStr.length >= 12 && !dateStr.includes('-') && !dateStr.includes('.')) {
                const y = dateStr.substring(0, 4);
                const m = dateStr.substring(4, 6);
                const d = dateStr.substring(6, 8);
                const h = dateStr.substring(8, 10);
                const min = dateStr.substring(10, 12);
                return `${d}.${m}.${y} ${h}:${min}`;
            }
            return formatSqlDate(dateStr);
        };

        let html = '';
        items.forEach((item, i) => {
            const id = item.EAMID || '-';
            const eqName = item.EquipmentName || '-';
            const EAMDocName = item.EAMDocName || '-';

            const rawStart = item.StartDateTime || '';
            const rawEnd = item.EndDateTime || '';
            const startFormatted = formatDisplayDate(rawStart);
            const endFormatted = formatDisplayDate(rawEnd);

            const typeName = item.DowntimeTypeName || '-';
            const author = item.Author || '-';
            const comment = item.Comment || '-';
            // console.log(`Простой #${i + 1}:`, { id, eqName, EAMDocName, startFormatted, endFormatted, typeName, author, comment });

            html += `
                <tr>
                    <td>${i + 1}</td>
                    <td>${EAMDocName}</td>
                    <td><span class="duration-badge">${eqName}</span></td>
                    <td>
                        <div style="font-size: 0.9em; display: flex; flex-direction: column; gap: 4px;">
                            <div><i class="fas fa-play" style="color: #4CAF50; width: 16px;"></i> ${startFormatted}</div>
                            <div><i class="fas fa-stop" style="color: #F44336; width: 16px;"></i> ${endFormatted}</div>
                        </div>
                    </td>
                    <td><span class="type-badge">${typeName}</span></td>
                    <td>${author}</td>
                    <td>-</td>
                    <td><div class="comment-cell" title="${comment}">${comment}</div></td>
                    <td style="text-align:right">
                        <button class="action-btn btn-edit" data-id="${id}" data-comment="${comment.replace(/"/g, '&quot;')}">
                            <i class="fas fa-pen"></i>
                        </button>
                    </td>
                </tr>
            `;
        });
        tableBody.innerHTML = html;

        document.querySelectorAll('.btn-edit').forEach(btn => {
            btn.onclick = () => {
                openEditModal(btn.dataset.id, btn.dataset.comment || "");
            };
        });
    } catch (e) {
        console.error("Ошибка загрузки данных таблицы:", e);
        tableBody.innerHTML = '<tr><td colspan="8" style="text-align: center; color: red;">Ошибка загрузки данных</td></tr>';
    }
}

function formatSqlDate(sqlDate) {
    if (!sqlDate) return "-";
    const safeDate = sqlDate.replace(' ', 'T');
    const d = new Date(safeDate);
    if (isNaN(d.getTime())) {
        return sqlDate.length > 16 ? sqlDate.substring(0, 16) : sqlDate;
    }
    const day = String(d.getDate()).padStart(2, '0');
    const month = String(d.getMonth() + 1).padStart(2, '0');
    const year = d.getFullYear();
    const hours = String(d.getHours()).padStart(2, '0');
    const mins = String(d.getMinutes()).padStart(2, '0');
    return `${day}.${month}.${year} ${hours}:${mins}`;
}

// Запуск при загрузке
// document.addEventListener('DOMContentLoaded', loadMainTableData);
document.addEventListener('DOMContentLoaded', () => {
    const params = new URLSearchParams(window.location.search);
    const dateVal = params.get('date');

    if (dateVal) {
        document.getElementById('filter-date-start').value = dateVal;
        loadMainTableData(dateVal);
    } else {
        loadMainTableData();
    }
});

// Логика кнопок быстрой очистки полей
document.querySelectorAll('.clear-btn').forEach(btn => {
    btn.onclick = function () {
        const container = this.closest('.search-select-container');
        const input = this.previousElementSibling; // Наш input

        input.value = ''; // Очищаем текст
        input.focus();    // Возвращаем фокус

        // Если это поле с поиском (есть скрытый ID), очищаем и его
        if (container) {
            const hiddenInput = container.querySelector('input[type="hidden"]');
            if (hiddenInput) hiddenInput.value = '';

            // Закрываем выпадающий список
            const dropdown = container.querySelector('.search-dropdown');
            if (dropdown) dropdown.style.display = 'none';
        }
    };
});

// ============================================
// ФУНКЦИИ ДЛЯ МОДАЛЬНОГО ОКНА ИНСТРУКЦИИ
// ============================================

function openInstructionModal() {
    const modal = document.getElementById('instructionModal');
    if (!modal) {
        console.error('Модальное окно инструкции не найдено');
        return;
    }
    modal.style.display = 'flex';
    // Анимация появления
    const content = modal.querySelector('.modal-content');
    if (content) {
        content.style.animation = 'none';
        // Принудительный рефлоу
        void content.offsetHeight;
        content.style.animation = 'modalSlide 0.4s cubic-bezier(0.18, 0.89, 0.32, 1.28)';
    }
}

function closeInstructionModal() {
    const modal = document.getElementById('instructionModal');
    if (!modal) {
        console.error('Модальное окно инструкции не найдено');
        return;
    }
    modal.style.display = 'none';
}

// Делаем функции глобальными
window.openInstructionModal = openInstructionModal;
window.closeInstructionModal = closeInstructionModal;

// ============================================
// ДОПОЛНИТЕЛЬНЫЕ ОБРАБОТЧИКИ ДЛЯ ИНСТРУКЦИИ
// ============================================

// Ждём загрузки DOM для добавления обработчиков
document.addEventListener('DOMContentLoaded', function() {
    // Обработчик кнопки инструкции
    const instructionBtn = document.getElementById('instructionBtn');
    if (instructionBtn) {
        instructionBtn.addEventListener('click', openInstructionModal);
    }
    
    // Обработчик крестика (на случай, если onclick в HTML не сработал)
    const closeBtn = document.querySelector('#instructionModal .close-modal');
    if (closeBtn) {
        closeBtn.addEventListener('click', closeInstructionModal);
    }
    
    // Закрытие по клику на фон
    const modal = document.getElementById('instructionModal');
    if (modal) {
        modal.addEventListener('click', function(event) {
            if (event.target === modal) {
                closeInstructionModal();
            }
        });
    }
    
    // Закрытие по Escape
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            const modal = document.getElementById('instructionModal');
            if (modal && modal.style.display === 'flex') {
                closeInstructionModal();
            }
        }
    });
});