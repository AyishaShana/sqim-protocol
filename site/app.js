const modal = document.querySelector("[data-modal]");
const openButtons = document.querySelectorAll("[data-open-waitlist]");
const closeButton = document.querySelector("[data-close-waitlist]");
const form = document.querySelector("[data-waitlist-form]");
const statusEl = document.querySelector("[data-form-status]");
const emailInput = document.querySelector("#email");

function openWaitlist() {
  modal.hidden = false;
  document.body.style.overflow = "hidden";
  window.setTimeout(() => emailInput.focus(), 40);
}

function closeWaitlist() {
  modal.hidden = true;
  document.body.style.overflow = "";
}

openButtons.forEach((button) => button.addEventListener("click", openWaitlist));
closeButton.addEventListener("click", closeWaitlist);

modal.addEventListener("click", (event) => {
  if (event.target === modal) {
    closeWaitlist();
  }
});

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !modal.hidden) {
    closeWaitlist();
  }
});

form.addEventListener("submit", (event) => {
  event.preventDefault();
  const email = new FormData(form).get("email").toString().trim();
  const waitlist = JSON.parse(localStorage.getItem("sqimWaitlist") || "[]");
  if (!waitlist.includes(email)) {
    waitlist.push(email);
    localStorage.setItem("sqimWaitlist", JSON.stringify(waitlist));
  }
  statusEl.textContent = "You're on the Sqim waitlist.";
  form.reset();
});
