const PHOTO_ICON = '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="8.5" cy="10" r="1.5"/><path d="m4 17 5-5 4 4 3-2 4 4"/></svg>';

const EQUIPMENT_PHOTOS = {
    // '3b831837-cc63-11ed-8163-00155d16fe68': '/static/img/mermec.webp',        // Устройство лазерное измерительное Mermec
    // '1f3d47e7-cee7-11ed-8163-00155d16fe68': '/static/img/glama_d3.webp',      // Манипулятор D3: Портал 3
    // '13d77b55-c172-11ed-8163-00155d16fe68': '/static/img/press_10k.webp',     // Пресс МН-10000 Schuler
    // 'c0489326-cd25-11ed-8163-00155d16fe68': '/static/img/kp.webp',            // Кольцевая печь
    // 'acff607c-be26-11ed-8163-00155d16fe68': '/static/img/glama_a1.webp',      // Портальный манипулятор Glama А1
    // '9dcbed00-c14a-11ed-8163-00155d16fe68': '/static/img/sggt.webp',          // Установка для удаления окалины SGGT
    // '7ffcb411-bc9a-11ed-8163-00155d16fe68': '/static/img/linsinger_saw.webp', // Пильная установка LINSINGER №1
    // '29cad886-c3bd-11ed-8163-00155d16fe68': '/static/img/linsinger_saw.webp', // Пильная установка LINSINGER №2
    // '9b7d1cf6-c222-11ed-8163-00155d16fe68': '/static/img/linsinger_saw.webp'  // Пильная установка LINSINGER №3
};

const STATUS = {
    run:  { label: 'В работе', color: '#16a34a' },
    wait: { label: 'Ожидание', color: '#f59e0b' },
    stop: { label: 'Авария',   color: '#ef4444' }
};

const REFRESH_INTERVAL_MS = 60000;

// Все 4 участка подключены к реальным данным.
const AREAS = [
    { slug: 'workpiece', name: 'Заготовительный участок',     connected: true },
    { slug: 'ring_oven',  name: 'Кольцевая печь',              connected: true },
    { slug: 'rolling',   name: 'Прессо-прокатный участок',    connected: true },
    { slug: 'thermal_section', name: 'Термоучасток',          connected: true }
];

const pad = n => String(n).padStart(2, '0');

function human(ms) {
    const m = Math.floor(ms / 60000);
    const d = Math.floor(m / 1440), h = Math.floor(m % 1440 / 60), mm = m % 60;
    if (d) return h ? `${d} сут ${h} ч` : `${d} сут`;
    if (h) return `${h} ч ${pad(mm)} мин`;
    return `${mm} мин`;
}

function parseSqlDate(str) {
    if (!str) return null;
    const d = new Date(str.replace(' ', 'T'));
    return isNaN(d.getTime()) ? null : d;
}

function formatHistoryDate(str) {
    if (!str) return '-';

    if (str.length >= 12 && !str.includes('-') && !str.includes('.')) {
        const y = str.substring(0, 4);
        const m = str.substring(4, 6);
        const d = str.substring(6, 8);
        const h = str.substring(8, 10);
        const min = str.substring(10, 12);
        return `${d}.${m}.${y} ${h}:${min}`;
    }

    const parsed = parseSqlDate(str);
    return parsed ? stamp(parsed) : '-';
}

function stamp(d) {
    return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()} ${clock(d)}`;
}

function clock(d) {
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function equipmentMarkup(eq) {
    const status = eq.Status || 'run';
    const photoUrl = EQUIPMENT_PHOTOS[(eq.EquipmentID || '').toLowerCase()];
    const photo = photoUrl
        ? `<img src="${photoUrl}" alt="${eq.EquipmentName || ''}" loading="lazy">`
        : PHOTO_ICON;

    return `
        <div class="mn-eq mn-eq--${status}" title="${STATUS[status].label}">
            <div class="mn-eq__photo">${photo}</div>
            <div class="mn-eq__name">${eq.EquipmentName || '-'}</div>
        </div>`;
}

function alarmMarkup(areaSlug, alarm) {
    if (!alarm) {
        return `<div class="mn-alarm" data-area="${areaSlug}"><p class="lbl">Последний аварийный простой</p><div class="mn-alarm__empty">Аварийных простоев не зафиксировано</div></div>`;
    }

    const start = parseSqlDate(alarm.StartDateTime);
    const end = parseSqlDate(alarm.EndDateTime);
    const active = !end;

    const meta = active
        ? `${start ? stamp(start) : '-'} <span>·</span> <b data-live="${start ? start.getTime() : ''}"></b>`
        : `${start ? stamp(start) : '-'} — ${clock(end)} <span>·</span> ${human(end - start)}`;

    return `
        <div class="mn-alarm" data-area="${areaSlug}">
            <div class="mn-alarm__head">
                <p class="lbl">Последний аварийный простой</p>
                ${active ? '' : `<span class="mn-alarm__free" data-elapsed="${end.getTime()}"></span>`}
            </div>
            <div class="mn-alarm__eq">${alarm.EquipmentName || '-'}</div>
            <div class="mn-alarm__reason">${alarm.DowntimeTypeName || alarm.Comment || '-'}</div>
            <div class="mn-alarm__meta">${meta}</div>
        </div>`;
}

function stubMarkup(area) {
    return `
        <section class="mn-area mn-area--stub">
            <h2 class="mn-area__name">${area.name}</h2>
            <div class="mn-area__list">
                <div class="mn-area__stub-msg">
                    <i class="fas fa-plug-circle-xmark"></i>
                    Участок пока не подключён
                </div>
            </div>
        </section>`;
}

function areaMarkup(area, data) {
    const equipment = data.Equipment || [];

    return `
        <section class="mn-area">
            <h2 class="mn-area__name">${area.name}</h2>
            <div class="mn-area__list">${equipment.map(equipmentMarkup).join('')}</div>
            ${alarmMarkup(area.slug, data.LastEmergency)}
        </section>`;
}

function renderFilters(allEquipment) {
    const totals = { run: 0, wait: 0, stop: 0 };
    allEquipment.forEach(eq => { totals[eq.Status || 'run']++; });

    document.getElementById('mn-filters').innerHTML = Object.entries(STATUS).map(([key, s]) => `
        <button type="button" class="mn-filter" data-filter="${key}">
            <span class="mn-filter__dot" style="background:${s.color}"></span>${s.label}
            <span class="mn-filter__count">${totals[key]}</span>
        </button>`).join('');
}

async function loadArea(area) {
    if (!area.connected) {
        return { area, data: null };
    }

    try {
        const response = await fetch(`/api/${area.slug}/monitoring`);

        if (!response.ok) {
            throw new Error(`Ошибка API ${response.status}`);
        }

        return { area, data: await response.json() };
    } catch (error) {
        console.error(`Ошибка загрузки участка ${area.slug}:`, error);
        return { area, data: null, failed: true };
    }
}

async function loadMonitoring() {
    const results = await Promise.all(AREAS.map(loadArea));

    const grid = document.getElementById('mn-grid');
    const allEquipment = [];

    grid.innerHTML = results.map(({ area, data, failed }) => {
        if (!area.connected) {
            return stubMarkup(area);
        }

        if (!data) {
            // Сеть подвела на этом тике — не рушим страницу и не спамим алертами.
            return failed
                ? `<section class="mn-area"><h2 class="mn-area__name">${area.name}</h2><div class="mn-loading mn-error">Нет связи с сервером</div></section>`
                : stubMarkup(area);
        }

        allEquipment.push(...(data.Equipment || []));
        return areaMarkup(area, data);
    }).join('');

    renderFilters(allEquipment);
    tick();
}

function tick() {
    const now = Date.now();
    const t = new Date();

    document.getElementById('clock-time').textContent = `${pad(t.getHours())}:${pad(t.getMinutes())}:${pad(t.getSeconds())}`;
    document.getElementById('clock-date').textContent = t.toLocaleDateString('ru-RU', { day: '2-digit', month: 'long', year: 'numeric' });

    document.querySelectorAll('[data-elapsed]').forEach(el => el.textContent = human(now - Number(el.dataset.elapsed)) + ' без аварий');
    document.querySelectorAll('[data-live]').forEach(el => el.textContent = human(now - Number(el.dataset.live)));
}

document.addEventListener('click', event => {
    const filterBtn = event.target.closest('.mn-filter');
    if (filterBtn) {
        const active = filterBtn.classList.contains('is-active');
        document.querySelectorAll('.mn-filter').forEach(b => b.classList.remove('is-active'));

        if (active) {
            delete document.body.dataset.filter;
        } else {
            filterBtn.classList.add('is-active');
            document.body.dataset.filter = filterBtn.dataset.filter;
        }
        return;
    }

    const alarmBlock = event.target.closest('.mn-alarm');
    if (alarmBlock && alarmBlock.dataset.area) {
        openHistoryModal(alarmBlock.dataset.area);
        return;
    }

    if (event.target.closest('#mn-modal-close') || event.target.id === 'mn-modal') {
        closeHistoryModal();
    }
});

let currentHistoryAreaSlug = null;

function openHistoryModal(areaSlug) {
    const area = AREAS.find(a => a.slug === areaSlug);
    currentHistoryAreaSlug = areaSlug;

    document.getElementById('mn-modal-title').textContent = `Простои — ${area ? area.name : areaSlug}`;
    document.getElementById('mn-modal').classList.add('is-open');

    const dateInput = document.getElementById('mn-modal-date');
    dateInput.value = '';
    document.getElementById('mn-modal-date-clear').classList.remove('is-visible');

    loadHistory(areaSlug);
}

function closeHistoryModal() {
    document.getElementById('mn-modal').classList.remove('is-open');
    currentHistoryAreaSlug = null;
}

async function loadHistory(areaSlug, dateVal = null) {
    const body = document.getElementById('mn-history-body');
    body.innerHTML = '<tr><td colspan="6" class="mn-loading">Загрузка данных...</td></tr>';

    try {
        let response;

        if (dateVal) {
            const dateForToir = dateVal.replaceAll('-', '');

            response = await fetch(`/api/monitoring/${areaSlug}/downtimes/filter`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    Filter_StartDate: dateForToir + '000000',
                    Filter_EndDate: dateForToir + '235959',
                    WithEAMDowntimes: 1,
                    ShowDeleted: 0,
                    MaxRows: 50,
                    Offset: 0
                })
            });
        } else {
            response = await fetch(`/api/monitoring/${areaSlug}/downtimes`);
        }

        if (!response.ok) {
            throw new Error(`Ошибка API ${response.status}`);
        }

        const data = await response.json();
        const items = Array.isArray(data) ? data : Object.values(data);

        if (items.length === 0) {
            body.innerHTML = '<tr><td colspan="6" class="mn-loading">Нет данных</td></tr>';
            return;
        }

        body.innerHTML = items.map(item => `
            <tr>
                <td><span class="mn-badge">${item.EAMDocName || '-'}</span></td>
                <td>${item.EquipmentName || '-'}</td>
                <td>
                    <div class="mn-period">
                        <div class="mn-period__row"><span class="mn-period__mark mn-period__mark--start"></span>${formatHistoryDate(item.StartDateTime)}</div>
                        <div class="mn-period__row"><span class="mn-period__mark mn-period__mark--end"></span>${formatHistoryDate(item.EndDateTime)}</div>
                    </div>
                </td>
                <td>${item.DowntimeTypeName || '-'}</td>
                <td>${item.Author || '-'}</td>
                <td>${item.Comment || '-'}</td>
            </tr>`).join('');
    } catch (error) {
        console.error('Ошибка загрузки истории простоев:', error);
        body.innerHTML = '<tr><td colspan="6" class="mn-error">Ошибка загрузки данных</td></tr>';
    }
}

document.getElementById('mn-modal-date').addEventListener('change', event => {
    const dateVal = event.target.value;
    document.getElementById('mn-modal-date-clear').classList.toggle('is-visible', Boolean(dateVal));

    if (currentHistoryAreaSlug) {
        loadHistory(currentHistoryAreaSlug, dateVal || null);
    }
});

document.getElementById('mn-modal-date-clear').addEventListener('click', () => {
    const dateInput = document.getElementById('mn-modal-date');
    dateInput.value = '';
    document.getElementById('mn-modal-date-clear').classList.remove('is-visible');

    if (currentHistoryAreaSlug) {
        loadHistory(currentHistoryAreaSlug);
    }
});

loadMonitoring();
setInterval(loadMonitoring, REFRESH_INTERVAL_MS);
setInterval(tick, 1000);