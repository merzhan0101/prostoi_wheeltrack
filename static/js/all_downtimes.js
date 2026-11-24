const body = document.getElementById('ad-body');
const sub = document.getElementById('ad-sub');
const dateInput = document.getElementById('ad-date');
const clearBtn = document.getElementById('ad-date-clear');
const refreshBtn = document.getElementById('ad-refresh');
const exportBtn = document.getElementById('ad-export');

// Данные приходят из БД и от пользователей — в HTML вставляем только экранированными.
const esc = value => String(value ?? '').replace(/[&<>"']/g, ch => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
}[ch]));

// YYYYMMDDHHMMSS -> дд.мм.гггг чч:мм
function formatDate(str) {
    if (!str) return '-';
    if (/^\d{12,14}$/.test(str)) {
        return `${str.substring(6, 8)}.${str.substring(4, 6)}.${str.substring(0, 4)} ${str.substring(8, 10)}:${str.substring(10, 12)}`;
    }
    return esc(str);
}

function formatDay(isoDate) {
    const [y, m, d] = isoDate.split('-');
    return `${d}.${m}.${y}`;
}

function pluralize(n) {
    const mod10 = n % 10, mod100 = n % 100;
    if (mod10 === 1 && mod100 !== 11) return 'запись';
    if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 'записи';
    return 'записей';
}

let requestId = 0;

async function load() {
    const dateVal = dateInput.value;
    const current = ++requestId;

    clearBtn.classList.toggle('is-visible', Boolean(dateVal));
    sub.textContent = 'Загрузка…';
    body.innerHTML = '<tr><td colspan="7" class="ad-loading">Загрузка данных...</td></tr>';

    try {
        const url = dateVal
            ? `/api/all-downtimes?date=${encodeURIComponent(dateVal.replaceAll('-', ''))}`
            : '/api/all-downtimes';

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Ошибка API ${response.status}`);
        }

        const data = await response.json();
        if (current !== requestId) return; // пришёл устаревший ответ

        const items = Array.isArray(data) ? data : Object.values(data);

        sub.textContent = dateVal
            ? `За ${formatDay(dateVal)}: ${items.length} ${pluralize(items.length)}`
            : `Последние ${items.length} ${pluralize(items.length)} по всем участкам`;

        if (items.length === 0) {
            body.innerHTML = '<tr><td colspan="7" class="ad-loading">Нет данных</td></tr>';
            return;
        }

        body.innerHTML = items.map(item => `
            <tr>
                <td class="ad-area">${esc(item.AreaName || '-')}</td>
                <td><span class="ad-badge">${esc(item.EAMDocName || '-')}</span></td>
                <td>${esc(item.EquipmentName || '-')}</td>
                <td>
                    <div class="ad-period">
                        <div class="ad-period__row"><span class="ad-period__mark ad-period__mark--start"></span>${formatDate(item.StartDateTime)}</div>
                        <div class="ad-period__row"><span class="ad-period__mark ad-period__mark--end"></span>${formatDate(item.EndDateTime)}</div>
                    </div>
                </td>
                <td>${esc(item.DowntimeTypeName || '-')}</td>
                <td>${esc(item.Author || '-')}</td>
                <td>${esc(item.Comment || '-')}</td>
            </tr>`).join('');
    } catch (error) {
        if (current !== requestId) return;
        console.error('Ошибка загрузки простоев всех участков:', error);
        sub.textContent = 'Ошибка загрузки';
        body.innerHTML = '<tr><td colspan="7" class="ad-error">Ошибка загрузки данных</td></tr>';
    }
}

dateInput.addEventListener('change', load);
refreshBtn.addEventListener('click', load);
clearBtn.addEventListener('click', () => {
    dateInput.value = '';
    load();
});

// Экспорт в Excel: тот же набор данных, что сейчас на экране (с учётом даты).
exportBtn.addEventListener('click', async () => {
    const dateVal = dateInput.value;
    const url = dateVal
        ? `/api/all-downtimes/export?date=${encodeURIComponent(dateVal.replaceAll('-', ''))}`
        : '/api/all-downtimes/export';

    exportBtn.disabled = true;

    try {
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Ошибка API ${response.status}`);
        }

        const blob = await response.blob();
        const disposition = response.headers.get('Content-Disposition') || '';
        const match = disposition.match(/filename="([^"]+)"/);

        const link = document.createElement('a');
        link.href = URL.createObjectURL(blob);
        link.download = match ? match[1] : 'downtimes.xlsx';
        document.body.appendChild(link);
        link.click();
        link.remove();
        URL.revokeObjectURL(link.href);
    } catch (error) {
        console.error('Ошибка экспорта в Excel:', error);
        alert('Не удалось сформировать файл Excel');
    } finally {
        exportBtn.disabled = false;
    }
});

load();