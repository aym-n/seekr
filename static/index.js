let outputDiv = document.querySelector('.result-list');

document.getElementById('submitButton').addEventListener('click', function() {
    let input = document.getElementById('inputField').value;
    fetch('/search', {
        method: 'POST',
        headers: {
        'Content-Type': 'application/json'
        },
        body: input,
    }).then(response => response.json())
    .then(data => {
        outputDiv.innerHTML = '';
        for(let i = 0; i < 10; i++) {
            let element = data[i];
            let div = document.createElement('div');
            div.className = 'result-container';
            outputDiv.appendChild(div);
    
            let a = document.createElement('a');
            a.href = '/'+element[0];
            a.className = 'result-link';
            a.innerHTML = element[0];
            div.appendChild(a);
    
            let span = document.createElement('span');
            span.className = 'tfidf-score';
            span.innerHTML = `Score: ${element[1]}`;
            div.appendChild(span);
        }
    });
});
