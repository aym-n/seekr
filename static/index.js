let outputDiv = document.getElementById('output');

document.getElementById('submitButton').addEventListener('click', function() {
    let input = document.getElementById('inputField').value;
    fetch('/', {
        method: 'POST',
        headers: {
        'Content-Type': 'application/json'
        },
        body: input,
    }).then(response => response.json())
    .then(data => {
        outputDiv.innerHTML = '';
        for (let i = 0; i < data.length; i++) {
            let p = document.createElement('p');
            p.innerHTML = data[i][0] + ' => ' + data[i][1];
            outputDiv.appendChild(p); 
        }
    });
});
