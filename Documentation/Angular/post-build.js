const fs = require('fs');
const path = require('path');

const indexPath = path.join(__dirname, 'dist', 'index.html');

console.log('Reading file:', indexPath);

fs.readFile(indexPath, 'utf8', (err, data) => {
  if (err) {
    return console.log('Error reading file:', err);
  }

  console.log('File read successfully');

  const result = data
    .replace(/href="\//g, 'href="./')
    .replace(/src="\//g, 'src="./');

  fs.writeFile(indexPath, result, 'utf8', (err) => {
    if (err) {
      return console.log('Error writing file:', err);
    }

    console.log('File written successfully');
  });
});
