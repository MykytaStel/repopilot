const express = require('express');
const app = express();
const logger = { log: () => {} };
app.get('/', (req, res) => {
  console.error('boom');
  console.warn('careful');
  logger.log('custom logger, not console.log');
  // console.log('debug left in a comment') must not trigger
  res.send('ok');
});
app.listen(3000);
