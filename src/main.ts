import { invoke } from "@tauri-apps/api/core";

type Settings={theme:string;fontSize:number;stopOnError:boolean;showKeyboard:boolean;showLiveWpm:boolean;adaptive:boolean;allowBackspace:boolean};
type Timing={totalMs:number;samples:number};
type PairStat={sessions:number;correct:number;errors:number;totalMs:number};
type Summary={timestampMs:number;mode:string;pairs:string[];wpm:number;accuracy:number;durationMs:number};
type AppData={settings:Settings;sessionsCompleted:number;totalPracticeMs:number;totalCorrect:number;totalErrors:number;pairStats:Record<string,PairStat>;keyErrors:Record<string,number>;transitionErrors:Record<string,number>;transitionCounts:Record<string,number>;keyTimings:Record<string,Timing>;transitionTimings:Record<string,Timing>;recentSessions:Summary[]};
type Plan={mode:string;pairs:string[];sections:{name:string;lines:string[]}[];seed:number};
type AccountStatus={hasAccounts:boolean;activeUsername:string|null};

const PAIRS=[
  {name:"Home row",items:[["fj","F / J"],["dk","D / K"],["sl","S / L"],["a;","A / ;"],["gh","G / H"]]},
  {name:"Top row",items:[["ru","R / U"],["ei","E / I"],["wo","W / O"],["qy","Q / Y"],["tp","T / P"]]},
  {name:"Bottom row",items:[["vm","V / M"],["c,","C / ,"],["x.","X / ."],["z/","Z / /"],["bn","B / N"]]}
] as const;
const $=<T extends HTMLElement>(s:string)=>document.querySelector<T>(s)!;
function make<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string,className?:string):HTMLElementTagNameMap[K]{const el=document.createElement(tag);if(text!==undefined)el.textContent=text;if(className)el.className=className;return el}
const groups=$("#pair-groups");
for(const group of PAIRS){const box=make("section",undefined,"pair-group");box.append(make("h2",group.name));for(const [value,label] of group.items){const row=make("label",undefined,"pair-option"),input=make("input");input.type="checkbox";input.value=value;row.append(input,make("span",label));box.append(row)}groups.append(box)}

let data:AppData;
let plan:Plan|null=null;
let flat:{section:string;line:string}[]=[];
let lineIndex=0, pos=0, typed:boolean[]=[], startedAt=0, lastAt=0, pausedAt=0, pausedTotal=0, correct=0, errors=0;
let keyErrors:Record<string,number>={},transitionErrors:Record<string,number>={},transitionCounts:Record<string,number>={},keyTimings:Record<string,Timing>={},transitionTimings:Record<string,Timing>={};
let previousExpected="";
let authMode:"login"|"create"="create",hasAccounts=false;
const exercise=$("#exercise"),metricEls={accuracy:$("#accuracy"),wpm:$("#wpm"),raw:$("#raw-wpm"),errors:$("#errors"),characters:$("#characters"),elapsed:$("#elapsed")};
let chars:HTMLSpanElement[]=[];

async function init(){
  try{const status=await invoke<AccountStatus>("account_status");hasAccounts=status.hasAccounts;showAuth(hasAccounts?"login":"create")}catch(e){showAuth("create");$("#auth-error").textContent=String(e)}
}
function showAuth(mode:"login"|"create") { authMode=mode;$("#auth-view").hidden=false;$("#app-shell").hidden=true;const creating=mode==="create";$("#auth-title").textContent=creating?"Create your account":"Welcome back";$("#auth-copy").textContent=creating?"Your password and progress stay on this computer.":"Sign in to continue your local progress.";$("#auth-submit").textContent=creating?"Create account":"Sign in";const confirm=$<HTMLInputElement>('[name="confirmPassword"]');$("#confirm-row").hidden=!creating;confirm.required=creating;$<HTMLInputElement>('[name="password"]').autocomplete=creating?"new-password":"current-password";$("#auth-switch").hidden=!hasAccounts;$("#auth-switch").textContent=creating?"Back to sign in":"Create another account";$("#auth-error").textContent="";$<HTMLInputElement>('[name="username"]').focus() }
function enterApp(username:string){$("#active-user").textContent=username;$("#auth-view").hidden=true;$("#app-shell").hidden=false;applySettings();fillSettings();renderProgress();showView("practice")}
$("#auth-form").addEventListener("submit",async e=>{e.preventDefault();const form=e.currentTarget as HTMLFormElement,username=(form.elements.namedItem("username") as HTMLInputElement).value.trim(),password=(form.elements.namedItem("password") as HTMLInputElement).value,confirm=(form.elements.namedItem("confirmPassword") as HTMLInputElement).value,submit=$<HTMLButtonElement>("#auth-submit");if(authMode==="create"&&password!==confirm){$("#auth-error").textContent="Passwords do not match";return}submit.disabled=true;$("#auth-error").textContent="";try{data=await invoke<AppData>(authMode==="create"?"create_account":"login",{username,password});hasAccounts=true;form.reset();enterApp(username)}catch(error){$("#auth-error").textContent=String(error)}finally{(form.elements.namedItem("password") as HTMLInputElement).value="";(form.elements.namedItem("confirmPassword") as HTMLInputElement).value="";submit.disabled=false}});
$("#auth-switch").addEventListener("click",()=>showAuth(authMode==="create"?"login":"create"));
$("#logout").addEventListener("click",async()=>{try{await invoke("logout");showAuth("login")}catch(error){console.error(error)}});
function selected(){return [...document.querySelectorAll<HTMLInputElement>('#pair-groups input:checked')].map(x=>x.value)}
function showError(message:string){$("#setup-error").textContent=message}
function updateSelectionUi(){const count=selected().length,total=PAIRS.reduce((sum,g)=>sum+g.items.length,0),all=$<HTMLInputElement>("#select-all"),start=$<HTMLButtonElement>("#start-practice");all.checked=count===total;all.indeterminate=count>0&&count<total;start.disabled=count===0;start.textContent=count===0?"Select a key pair":count===1?"Start standalone practice":"Start mixed practice";$("#selection-count").textContent=`${count} selected`;showError("")}
async function start(){
  const pairs=selected();if(!pairs.length)return;const mode=pairs.length===1?"standalone":"mixed";
  showError("");
  let generated:Plan;try{generated=await invoke<Plan>("generate_session",{request:{pairs,mode,length:$<HTMLSelectElement>("#length").value,adaptive:data.settings.adaptive}})}catch(e){return showError(String(e))}
  plan=generated;flat=generated.sections.flatMap(s=>s.lines.map(line=>({section:s.name,line})));resetSession();$("#setup").hidden=true;$("#results").hidden=true;$("#session").hidden=false;renderKeyboard();renderLine();$("#session").focus();
}
function resetSession(){lineIndex=pos=correct=errors=pausedTotal=0;typed=[];startedAt=performance.now();lastAt=0;pausedAt=0;keyErrors={};transitionErrors={};transitionCounts={};keyTimings={};transitionTimings={};previousExpected=""}
function renderLine(){const item=flat[lineIndex];$("#section-name").textContent=`${item.section} · ${lineIndex+1} / ${flat.length}`;$("#session-title").textContent=plan!.mode==="standalone"?`${labelFor(plan!.pairs[0])} standalone`:`${plan!.pairs.map(labelFor).join(" + ")} mixed`;
  chars=[...item.line].map((char,i)=>{const span=make("span",char,"char"+(i===0?" current":""));return span});exercise.replaceChildren(...chars);updateMetrics()}
function activeMs(){return Math.max(0,(pausedAt||performance.now())-startedAt-pausedTotal)}
function metrics(){const chars=correct+errors,minutes=activeMs()/60000;return{chars,wpm:minutes?correct/5/minutes:0,raw:minutes?chars/5/minutes:0,accuracy:chars?correct/chars*100:100}}
function setText(el:HTMLElement,text:string){if(el.textContent!==text)el.textContent=text}
function updateMetrics(){const m=metrics();setText(metricEls.accuracy,`${m.accuracy.toFixed(1)}%`);setText(metricEls.wpm,data.settings.showLiveWpm?m.wpm.toFixed(0):"—");setText(metricEls.raw,data.settings.showLiveWpm?m.raw.toFixed(0):"—");setText(metricEls.errors,String(errors));setText(metricEls.characters,String(m.chars));const sec=Math.floor(activeMs()/1000);setText(metricEls.elapsed,`${Math.floor(sec/60)}:${String(sec%60).padStart(2,"0")}`)}
function showTyped(ok:boolean){chars[pos].className=`char ${ok?"correct":"incorrect"}`;pos++;chars[pos]?.classList.add("current");updateMetrics()}
function addTiming(map:Record<string,Timing>,key:string,ms:number){if(ms<20||ms>3000)return;const t=map[key]??={totalMs:0,samples:0};t.totalMs+=Math.round(ms);t.samples++}

document.addEventListener("keydown",async e=>{
  if($("#session").hidden)return;
  if(e.key==="Escape"){e.preventDefault();exitSession();return}
  if(pausedAt)return;
  const line=flat[lineIndex].line;
  if(pos>=line.length){if(e.key==="Enter"){e.preventDefault();await nextLine()}return}
  if(e.key==="Backspace"){if(data.settings.allowBackspace&&pos>0){e.preventDefault();pos--;const wasCorrect=typed.pop()!;wasCorrect?correct--:errors--;chars[pos].className="char current";updateMetrics()}return}
  if(e.ctrlKey||e.altKey||e.metaKey||e.key.length!==1)return;
  e.preventDefault();const expected=line[pos],ok=e.key===expected;const now=performance.now(),delay=lastAt?now-lastAt:0;
  if(previousExpected){const edge=previousExpected+expected;transitionCounts[edge]=(transitionCounts[edge]||0)+1}
  if(!ok){errors++;keyErrors[expected]=(keyErrors[expected]||0)+1;if(previousExpected){const edge=previousExpected+expected;transitionErrors[edge]=(transitionErrors[edge]||0)+1}if(data.settings.stopOnError){updateMetrics();return}}
  else correct++;
  if(delay){addTiming(keyTimings,expected,delay);if(previousExpected)addTiming(transitionTimings,previousExpected+expected,delay)}
  typed.push(ok);previousExpected=expected;lastAt=now;showTyped(ok);
});
async function nextLine(){lineIndex++;if(lineIndex>=flat.length){const length=$("#length");if(length instanceof HTMLSelectElement&&length.value==="endless"){const extra=await invoke<Plan>("generate_session",{request:{pairs:plan!.pairs,mode:plan!.mode,length:"medium",adaptive:data.settings.adaptive}});flat.push(...extra.sections.flatMap(s=>s.lines.map(line=>({section:s.name,line}))))}else{return finish()}}pos=0;typed=[];previousExpected="";renderLine()}
async function finish(){const m=metrics(),duration=Math.round(activeMs());$("#session").hidden=true;$("#results").hidden=false;resultSummary(m,duration);rank($("#missed-keys"),keyErrors," errors");rank($("#problem-transitions"),transitionErrors," errors",transitionLabel);rankTime($("#slow-keys"),keyTimings);rankTime($("#slow-transitions"),transitionTimings,transitionLabel);
  try{data=await invoke<AppData>("record_session",{record:{timestampMs:Date.now(),mode:plan!.mode,pairs:plan!.pairs,wpm:m.wpm,accuracy:m.accuracy,durationMs:duration,correct,errors,keyErrors,transitionErrors,transitionCounts,keyTimings,transitionTimings}});renderProgress()}catch(e){console.error(e)}
}
function statGrid(el:HTMLElement,items:(string|number)[][]){el.replaceChildren(...items.map(([value,label])=>{const box=make("div");box.append(make("strong",String(value)),make("span",String(label)));return box}))}
function resultSummary(m:ReturnType<typeof metrics>,duration:number){statGrid($("#result-summary"),[[m.accuracy.toFixed(1)+"%","accuracy"],[m.wpm.toFixed(0),"wpm"],[m.raw.toFixed(0),"raw wpm"],[String(m.chars),"keystrokes"],[String(correct),"correct"],[formatDuration(duration),"duration"]])}
function rank(el:HTMLElement,map:Record<string,number>,suffix:string,format=(x:string)=>x.toUpperCase()){const rows=Object.entries(map).sort((a,b)=>b[1]-a[1]).slice(0,5);el.replaceChildren(...(rows.length?rows.map(([k,v])=>make("li",`${format(k)} · ${v}${suffix}`)):[make("li","None — excellent control")]))}
function rankTime(el:HTMLElement,map:Record<string,Timing>,format=(x:string)=>x.toUpperCase()){const rows=Object.entries(map).filter(([,v])=>v.samples>=2).sort((a,b)=>b[1].totalMs/b[1].samples-a[1].totalMs/a[1].samples).slice(0,5);el.replaceChildren(...(rows.length?rows.map(([k,v])=>make("li",`${format(k)} · ${Math.round(v.totalMs/v.samples)} ms`)):[make("li","Not enough samples yet")]))}
function transitionLabel(x:string){return [...x].map(c=>c.toUpperCase()).join(" → ")}
function exitSession(){if(!confirm("End this session without saving it?"))return;$("#session").hidden=true;$("#setup").hidden=false}

function fillSettings(){const f=$("#settings-form") as HTMLFormElement;for(const [k,v] of Object.entries(data.settings)){const input=f.elements.namedItem(k) as HTMLInputElement|HTMLSelectElement|null;if(input)typeof v==="boolean"?(input as HTMLInputElement).checked=v:input.value=String(v)}}
function applySettings(){document.documentElement.dataset.theme=data.settings.theme;document.documentElement.style.setProperty("--font-size",`${data.settings.fontSize}px`)}
$("#settings-form").addEventListener("change",async()=>{const f=$("#settings-form") as HTMLFormElement;const val=(n:string)=>(f.elements.namedItem(n) as HTMLInputElement);data.settings={theme:val("theme").value,fontSize:Number(val("fontSize").value),stopOnError:val("stopOnError").checked,showKeyboard:val("showKeyboard").checked,showLiveWpm:val("showLiveWpm").checked,adaptive:val("adaptive").checked,allowBackspace:val("allowBackspace").checked};applySettings();try{await invoke("save_settings",{settings:data.settings})}catch(e){console.error(e)}});
function renderKeyboard(){const el=$("#keyboard-guide");el.hidden=!data.settings.showKeyboard;if(el.hidden)return;const keys="qwertyuiopasdfghjkl;zxcvbnm,./",on=new Set(plan!.pairs.flatMap(p=>[...p]));el.replaceChildren(...[...keys].map(k=>make("span",k.toUpperCase(),`keycap ${on.has(k)?"on":""}`)))}
function renderProgress(){const total=data.totalCorrect+data.totalErrors,accuracy=total?data.totalCorrect/total*100:100;const avgWpm=data.recentSessions.length?data.recentSessions.reduce((s,x)=>s+x.wpm,0)/data.recentSessions.length:0;statGrid($("#overall"),[[data.sessionsCompleted,"sessions"],[formatDuration(data.totalPracticeMs),"practice time"],[accuracy.toFixed(1)+"%","overall accuracy"],[avgWpm.toFixed(0),"recent avg wpm"]]);
  const pairRows=Object.entries(data.pairStats).sort((a,b)=>b[1].sessions-a[1].sessions),pairProgress=$("#pair-progress");pairProgress.replaceChildren(...(pairRows.length?pairRows.map(([p,s])=>{const a=s.correct+s.errors?s.correct/(s.correct+s.errors)*100:100,w=s.totalMs?s.correct/5/(s.totalMs/60000):0,row=make("div",undefined,"pair-stat");row.append(make("strong",labelFor(p)),make("span",`${s.sessions} sessions · ${a.toFixed(1)}% · ${w.toFixed(0)} WPM`));return row}):[make("p","Complete a session to begin tracking progress.")]));
  const weakKeys=Object.entries(data.keyErrors).sort((a,b)=>b[1]-a[1]).slice(0,5),weakTrans=Object.entries(data.transitionErrors).sort((a,b)=>b[1]-a[1]).slice(0,5);$("#weak-spots").replaceChildren(make("p",`Keys: ${weakKeys.map(([k,v])=>`${k.toUpperCase()} (${v})`).join(", ")||"none yet"}`),make("p",`Transitions: ${weakTrans.map(([k,v])=>`${transitionLabel(k)} (${v})`).join(", ")||"none yet"}`));
  const recent=$("#recent"),sessions=data.recentSessions.slice(0,8);recent.replaceChildren(...(sessions.length?sessions.map(s=>{const row=make("div",undefined,"recent-row");row.append(make("strong",s.pairs.map(labelFor).join(" + ")),make("span",`${s.accuracy.toFixed(1)}% · ${s.wpm.toFixed(0)} WPM`));return row}):[make("p","No sessions yet.")]))}
function formatDuration(ms:number){const mins=Math.floor(ms/60000),secs=Math.floor(ms/1000)%60;return mins?`${mins}m ${secs}s`:`${secs}s`}
function labelFor(pair:string){return pair.split("").map(x=>x.toUpperCase()).join(" / ")}
function showView(name:string){document.querySelectorAll(".view").forEach(x=>x.classList.toggle("active",x.id===`${name}-view`));document.querySelectorAll(".nav").forEach(x=>x.classList.toggle("active",(x as HTMLElement).dataset.view===name));if(name==="progress")renderProgress()}
document.querySelectorAll<HTMLElement>(".nav").forEach(x=>x.addEventListener("click",()=>showView(x.dataset.view!)));
groups.addEventListener("change",updateSelectionUi);$<HTMLInputElement>("#select-all").addEventListener("change",e=>{document.querySelectorAll<HTMLInputElement>("#pair-groups input").forEach(input=>input.checked=(e.currentTarget as HTMLInputElement).checked);updateSelectionUi()});$("#start-practice").addEventListener("click",start);$("#exit-session").addEventListener("click",exitSession);$("#practice-again").addEventListener("click",()=>{$("#results").hidden=true;$("#setup").hidden=false});
init();
