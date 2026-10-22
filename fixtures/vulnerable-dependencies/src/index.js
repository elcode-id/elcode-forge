const lodash = require('lodash');
const minimist = require('minimist');

function parseArgs(args) {
    return minimist(args);
}

module.exports = { parseArgs };
