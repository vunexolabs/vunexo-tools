-- Round 3: core V1 schema. See docs/expense-manager/database-schema.md §11
-- for the full design rationale (snapshot strategy, money representation,
-- receipt storage, delete rules) — this file is the literal SQL from that
-- locked design, not the place to relitigate it.

CREATE TABLE business (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    address TEXT,
    tax_info TEXT,
    currency_symbol TEXT NOT NULL DEFAULT '₹',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE vendors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    contact TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    default_deductible INTEGER NOT NULL DEFAULT 0, -- boolean
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE expenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    tax_amount_minor INTEGER NOT NULL DEFAULT 0,
    itc_eligible INTEGER NOT NULL DEFAULT 0, -- boolean
    deductible INTEGER NOT NULL DEFAULT 0,   -- boolean, snapshot from category at creation
    payment_method TEXT NOT NULL,
    notes TEXT,
    receipt_path TEXT,
    vendor_id INTEGER REFERENCES vendors(id),
    vendor_name_snapshot TEXT,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    category_name_snapshot TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_expenses_category ON expenses(category_id);
CREATE INDEX idx_expenses_vendor ON expenses(vendor_id);
CREATE INDEX idx_expenses_date ON expenses(date);
