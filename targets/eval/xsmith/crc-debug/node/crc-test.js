const { crc32 } = require('crc');

var crc_start = true;
var crc_value = 0;

function addToCrc(val) {
    var buf;
    if (typeof val === 'number') {
        buf = new Buffer.alloc(4);
        buf.writeUInt32LE(val);
    } else {
        console.log('Could not crc given value. The value was not an integer');
        return;
    }
    //console.log(`buffer: ${buf.toString('hex')}  previous: ${crc_value.toString(16)}`)
    if(crc_start) {
      crc_value = crc32(buf);
      crc_start = false;
    } else {
      crc_value = crc32(buf, crc_value);
    }
}

crc_start = true
addToCrc(123);
addToCrc(456);
addToCrc(789);

console.log(crc_value.toString(16));

