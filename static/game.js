var ctx = null;
var dirty = true;
var cardData = [];
var cardSize = {
  width: 80,
  height: 120,
};
var cardPos = [
  {x: 290, y: 60},
  {x: 430, y: 60},
  {x: 220, y: 240},
  {x: 360, y: 240},
  {x: 500, y: 240},
  {x: 290, y: 420},
  {x: 430, y: 420},
];
var colors = {
  r: { c: "#FF0000", x:  9, y:  8},
  o: { c: "#FFBF00", x: 46, y:  8},
  y: { c: "#FFE135", x:  9, y: 44},
  g: { c: "#00FF00", x: 46, y: 44},
  b: { c: "#0000FF", x:  9, y: 80},
  v: { c: "#8A2BE2", x: 46, y: 80},
  circle: { c: "#000000" },
  hover:  { c: "#D3212D" },
  select: { c: "#FFFFFF" },
  unselect: { c: "#D1D1D1" },
};

function start() {
  var canvas = document.getElementById('game');
  ctx = canvas.getContext('2d');
  if (ctx) {
    setupMouse(canvas);
    getCards();
  } else {
    alert("Couldn't get the drawing context");
  }
  pollUpdates();
}

function pollUpdates() {
  console.log("starting to poll...");
  get_info("/update", function(res) {
    if (res) {
      log("Set found! Updating...");
      getCards();
    } else {
      console.log("nothing to update");
      console.log(res);
    }
    console.log("finished updating!");
    return pollUpdates();
  });
}

function newGame() {
  get_info("/new", function(result) {
    if (result.ok) {
      getCards();
      log("started new game");
    } else {
      alert("Creating new game failed");
    }
  });
}

function submit() {
  var cards = [];
  cardData.forEach(function (card) {
    if (card.selected) {
      dirty = true;
      card.selected = false;
      cards.push(card.n);
    }
  });
  render();
  console.log("sending", cards);
  post_info("/submit", cards, function(result) {
    if (result.success === null) {
      log("Error checking set");
      log(result.msg);
    } else if (result.success) {
      log("correct");
      getCards();
    } else {
      log("incorrect");
    }
  });
}

function setupMouse(canvas) {
  var getMousePos = function(evt) {
    var rect = canvas.getBoundingClientRect();
    return {
      x: evt.clientX - rect.left,
      y: evt.clientY - rect.top
    }
  }

  canvas.addEventListener('mousemove', function(evt) {
    var mousePos = getMousePos(evt);
    cardData.forEach(function (card, index) {
      if (mousePos.x > cardPos[index].x && mousePos.x < cardPos[index].x + cardSize.width) {
        if (mousePos.y > cardPos[index].y && mousePos.y < cardPos[index].y + cardSize.height) {
          dirty = true;
          card.hovered = true;
          return;
        }
      }
      if (card.hovered) {
        dirty = true;
        card.hovered = false;
      }
    });
    render();
  }, false);

  canvas.addEventListener('click', function (evt) {
    var mousePos = getMousePos(evt);
    cardData.forEach(function (card, index) {
      if (mousePos.x > cardPos[index].x && mousePos.x < cardPos[index].x + cardSize.width) {
        if (mousePos.y > cardPos[index].y && mousePos.y < cardPos[index].y + cardSize.height) {
          dirty = true;
          card.selected = !card.selected;
          return;
        }
      }
    });
    if (!dirty) { // no card was selected
      dirty = true;
      cardData.forEach(function (card) {
        card.selected = false;
      });
    }
    render();
  }, false);
}

function getCards() {
  get_info("/cards", function(cardData) {
    if (cardData.length == 0) {
      console.log("click new game");
      log("click new game");
    }
    setCardData(cardData);
    dirty = true;
    render();
  });
}

function setCardData(data) {
  arr = [];
  data.forEach(function (num) {
    arr.push({
      n: num,
      r: (num & 32) == 32,
      o: (num & 16) == 16,
      y: (num &  8) ==  8,
      g: (num &  4) ==  4,
      b: (num &  2) ==  2,
      v: (num &  1) ==  1,
      hovered: false,
      selected: false,
    });
  });
  cardData = arr;
}

var logsInfo = {
  age : 6000,
  logs : [],
  colors : {
    major: "#000000",
    minor: "#555555",
  },
}
function log(str) {
  var l = logsInfo.logs;
  l.unshift(str);
  dirty = true;
  render();
  // clean up old logs
  if (l[1]) {
    setTimeout(function() { l.pop(); dirty = true; render(); }, logsInfo.age);
  }
}

function render() {
  if (!dirty) return;
  ctx.clearRect(0,0,800,600);
  if (logsInfo.logs.length) {
    var old_font = ctx.font;
    var old_fill = ctx.fillStyle;
    ctx.font="20px Georgia";
    logsInfo.logs.forEach(function (msg, i) {
      if (i == 0) ctx.fillStyle = logsInfo.colors.major;
      else ctx.fillStyle = logsInfo.colors.minor;
      var y = 590 - i * 25;
      if (y > 0) ctx.fillText(msg, 600, y);
    });
    ctx.fillText(">", 580, 590);
    ctx.font = old_font;
    ctx.fillStyle = old_fill;
  }
  cardData.forEach(function(card, index) {
    var pos = cardPos[index];
    var old_stroke = ctx.strokeStyle;
    var old_fill = ctx.fillStyle;
    if (card.hovered) {
      ctx.strokeStyle = colors.hover.c;
    }
    if (card.selected) {
      ctx.fillStyle = colors.select.c;
    } else {
      ctx.fillStyle = colors.unselect.c;
    }
    ctx.fillRect(pos.x, pos.y, cardSize.width, cardSize.height);
    ctx.strokeRect(pos.x, pos.y, cardSize.width, cardSize.height);
    ctx.strokeStyle = old_stroke;
    ctx.fillgStyle = old_stroke;
    ["r","o","y","g","b","v"].forEach(function (col) {
      if (card[col]) {
        var c = colors[col];
        var x = pos.x + c.x + 14;
        var y = pos.y + c.y + 14;
        c = c.c;
        var old_fill = ctx.fillStyle;
        var old_stroke = ctx.strokeStyle;
        ctx.fillStyle = c;
        ctx.strokeStyle = colors.circle;
        ctx.beginPath();
        ctx.arc(x,y,14,0,2*Math.PI);
        ctx.fill();
        ctx.beginPath();
        ctx.arc(x,y,14,0,2*Math.PI);
        ctx.stroke();
        ctx.fillStyle = old_fill;
        ctx.strokeStyle = old_stroke;
      }
    });
  });
  dirty = false;
}

function get_info(endpoint, callback) {
  fetch(encodeURI(endpoint), {
    method: "GET",
  }).then((response) => response.json())
    .then((data) => callback(data))
    .catch((error) => console.log('Error:', error))
}

function post_info(endpoint, data, callback) {
  fetch(endpoint, {
    method: "POST",
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  }).then((response) => response.json())
    .then((data) => callback(data))
    .catch((error) => console.log('Error:', error))
}
    
