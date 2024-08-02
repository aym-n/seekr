fetch('http://0.0.0.0:8000/search', {
    method: 'POST',
    body: JSON.stringify({ text: 'Hello, world!' }),
    headers: {
        'Content-Type': 'application/json'
    }
})
    .then(response => response.json())
    .then(data => console.log(data))
    .catch(error => console.error(error));
