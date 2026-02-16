-- Returns transactions where created or spent UTXOs span multiple distinct intent_hashes.
-- Only these transactions are affected by HashMap vs BTreeMap ordering differences.
-- Output includes tx_hash and block_height for use as override data.
WITH multi_intent_created AS (
  SELECT creating_transaction_id AS tx_id
  FROM unshielded_utxos
  GROUP BY creating_transaction_id
  HAVING COUNT(DISTINCT intent_hash) > 1
),
multi_intent_spent AS (
  SELECT spending_transaction_id AS tx_id
  FROM unshielded_utxos
  WHERE spending_transaction_id IS NOT NULL
  GROUP BY spending_transaction_id
  HAVING COUNT(DISTINCT intent_hash) > 1
),
interesting_txs AS (
  SELECT tx_id FROM multi_intent_created
  UNION
  SELECT tx_id FROM multi_intent_spent
),
created_utxos AS (
  SELECT
    u.creating_transaction_id AS tx_id,
    json_agg(
      json_build_object(
        'intent_hash', encode(u.intent_hash, 'hex'),
        'output_index', u.output_index
      ) ORDER BY u.id
    ) AS utxos
  FROM unshielded_utxos u
  WHERE u.creating_transaction_id IN (SELECT tx_id FROM interesting_txs)
  GROUP BY u.creating_transaction_id
),
spent_utxos AS (
  SELECT
    u.spending_transaction_id AS tx_id,
    json_agg(
      json_build_object(
        'intent_hash', encode(u.intent_hash, 'hex'),
        'output_index', u.output_index
      ) ORDER BY u.id
    ) AS utxos
  FROM unshielded_utxos u
  WHERE u.spending_transaction_id IN (SELECT tx_id FROM interesting_txs)
  GROUP BY u.spending_transaction_id
)
SELECT COALESCE(json_agg(row_data ORDER BY row_data->>'block_height'), '[]'::json)
FROM (
  SELECT
    json_build_object(
      'tx_hash', encode(t.hash, 'hex'),
      'block_height', b.height,
      'created', COALESCE(c.utxos, '[]'::json),
      'spent', COALESCE(s.utxos, '[]'::json)
    ) AS row_data
  FROM interesting_txs it
  JOIN transactions t ON t.id = it.tx_id
  JOIN blocks b ON b.id = t.block_id
  LEFT JOIN created_utxos c ON c.tx_id = it.tx_id
  LEFT JOIN spent_utxos s ON s.tx_id = it.tx_id
) sub;
