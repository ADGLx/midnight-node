const express = require('express');
const bodyParser = require('body-parser');
const { execFile } = require('child_process');
const path = require('path');

const app = express();
const PORT = process.env.PORT || 3000;

app.use(bodyParser.json());
app.use(express.static(path.join(__dirname)));

app.post('/api/faucet', (req, res) => {
  const { address } = req.body;
  if (!address || typeof address !== 'string') {
    return res.status(400).json({ error: 'Invalid Cardano address.' });
  }

  // Call the fund_address.sh script with the provided address and a fixed amount (e.g., 1500000 lovelace)
  const scriptPath = path.join(__dirname, 'fund_address.sh');
  const amount = '10'; // You can adjust this value as needed
  execFile(scriptPath, [address, amount], (error, stdout, stderr) => {
    console.log('fund_address.sh stdout:\n', stdout);
    console.log('fund_address.sh stderr:\n', stderr);
    if (error) {
      return res.status(500).json({ error: stderr || error.message });
    }
    return res.json({ message: `Successfully funded address ${address} with ${amount} tcNight.` });
  });
});

app.listen(PORT, () => {
  console.log(`Cnight faucet server running on port ${PORT}`);
});
