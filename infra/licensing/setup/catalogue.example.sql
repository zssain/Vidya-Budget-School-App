-- Operator template (NOT a migration): define what the public checkout sells.
-- NO REAL PRICE SHIPS WITH THIS CODE. The owner decides the SKU, name, price and
-- modules; replace every <placeholder>. Price is INTEGER PAISE (₹1 = 100 paise).
-- Orders snapshot price + modules, so later edits never change an existing purchase.
-- To change a price, insert a NEW immutable SKU (e.g. …-v2) and deactivate the old one.
--
-- Allowed module keys = 'core' + the app's toggleable module keys (the signer rejects
-- anything else; see docs/implementation/licence-signer.md). 'core' is mandatory.

INSERT INTO billing.products (id, name, currency, price_paise, modules)
VALUES ('<sku-and-version, e.g. vidya-perpetual-v1>', '<display name>', 'INR', <price_in_paise>, ARRAY['core']);

-- Retire a SKU (existing orders are unaffected):
-- UPDATE billing.products SET active = false WHERE id = '<sku>';
