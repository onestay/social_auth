
document.addEventListener('DOMContentLoaded', main)

function twitchConnect() {
    window.location.replace("/twitch/authorize")
}

function twitterConnect() {
    window.location.replace("/twitter/authorize")
}

function main() {
    fetch('/api/v1/avail', {
        headers: {
            "Authorization": "abc"
        }
    })
    .then(res => res.json())
    .then((json) => {
        if (json.services.twitch) {
            let button = document.getElementById('twitchButton');
            button.classList.remove('is-loading')
            button.setAttribute('disabled', 'true')
            button.innerHTML = 'Already connected'
        } else {
            let button = document.getElementById('twitchButton');
            button.classList.remove('is-loading')
        }

        if (json.services.twitter) {
            let button = document.getElementById('twitterButton');
            button.classList.remove('is-loading')
            button.setAttribute('disabled', 'true')
            button.innerHTML = 'Already connected'
        } else {
            let button = document.getElementById('twitterButton');
            button.classList.remove('is-loading')
        }


    })
    .catch(e => console.error(e))
}