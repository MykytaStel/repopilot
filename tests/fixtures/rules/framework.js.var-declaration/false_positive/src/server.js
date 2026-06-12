const express = require('express');
const app = express();
const handlerVar = 1;
let activeRoutes = handlerVar;
// var legacy = 1; a commented-out var must not trigger
app.listen(3000);
