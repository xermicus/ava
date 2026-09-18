/* The header of a sortable table sorts it. The column and its direction are
 * kept by the name of the table, since a refresh replaces the table with the
 * one the server sorted. A blank cell sorts last either way. */
const VALUE_FIELD = "[data-value]";
const sorts = new Map();
const DESCENDING = "desc";
const ASCENDING = "asc";
const NUMERIC = "numeric";

function sortTable(table) {
    const column = Number(table.dataset.sorted);
    const headers = [...table.tHead.rows[0].cells];
    if (!headers[column]) {
        return;
    }

    const descending = table.dataset.order === DESCENDING;
    const body = table.tBodies[0];
    const rows = [...body.rows].filter((row) => row.cells.length === headers.length);
    /* A label carrying a unit, an hour or a million tokens, is another number
     * read as text, so the cell says what to sort it by, and a column of such
     * cells is numeric whatever its header says. A dollar figure is a number
     * with a sign in front of it. */
    const valued = (row) => row.cells[column].querySelector(VALUE_FIELD);
    const key = (row) => {
        const value = valued(row);
        return value
            ? value.dataset.value
            : row.cells[column].textContent.trim().replace(/[$,]/g, "");
    };
    const numeric = headers[column].dataset.sort === NUMERIC || rows.some(valued);

    /* A cell holding no number sorts last either way, the blank ones and the
     * cells saying a pairing saw no fight alike. */
    rows.sort((left, right) => {
        const first = key(left);
        const second = key(right);
        if (numeric) {
            const one = Number.parseFloat(first);
            const other = Number.parseFloat(second);
            if (!Number.isFinite(one) || !Number.isFinite(other)) {
                return Number.isFinite(one) ? -1 : Number.isFinite(other) ? 1 : 0;
            }
            return descending ? other - one : one - other;
        }
        if (first === "" || second === "") {
            return first === second ? 0 : first === "" ? 1 : -1;
        }
        return descending ? -first.localeCompare(second) : first.localeCompare(second);
    });
    body.append(...rows);

    for (const [index, header] of headers.entries()) {
        const arrow = header.querySelector("[data-arrow]");
        if (arrow) {
            arrow.textContent = index === column ? (descending ? "↓" : "↑") : "";
        }
    }
}

function restoreSorts() {
    for (const table of document.querySelectorAll("table[data-sortable]")) {
        const sort = sorts.get(table.dataset.sortable);
        if (!sort) {
            continue;
        }
        table.dataset.sorted = sort.column;
        table.dataset.order = sort.order;
        sortTable(table);
    }
}

document.addEventListener("click", (event) => {
    const header = event.target.closest("th[data-sort]");
    const table = header?.closest("table[data-sortable]");
    if (!table) {
        return;
    }

    const column = String([...header.parentElement.cells].indexOf(header));
    const order =
        table.dataset.sorted === column
            ? (table.dataset.order === DESCENDING ? ASCENDING : DESCENDING)
            : (header.dataset.sort === NUMERIC ? DESCENDING : ASCENDING);

    table.dataset.sorted = column;
    table.dataset.order = order;
    sorts.set(table.dataset.sortable, { column, order });
    sortTable(table);
});
