export const DAY = 86400000;
export const random = (n=32) => [...crypto.getRandomValues(new Uint8Array(n))].map(x=>x.toString(16).padStart(2,'0')).join('');
export const hex = bytes => [...bytes].map(x=>x.toString(16).padStart(2,'0')).join('');
export const hash = async value => hex(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(value))));
export const json = value => Response.json(value);
export class ApiError extends Error { constructor(code,status=400,message=code.replaceAll('_',' ')) {super(message);this.code=code;this.status=status;} }
export function fail(code,status=400) {throw new ApiError(code,status);}
export const stmt = (env,sql,...args) => env.DB.prepare(sql).bind(...args);
export async function body(request,limit=131072) {
 const bytes=await bounded(request,limit);
 try { const v=JSON.parse(new TextDecoder().decode(bytes));if(!v || Array.isArray(v)||typeof v!=='object')fail('invalid_request'); return v; } catch { fail('invalid_request'); }
}
export async function bounded(request,limit) {
 if(Number(request.headers.get('Content-Length'))>limit)fail('body_too_large',413);
 const reader=request.body?.getReader();if(!reader)return new Uint8Array();
 const chunks=[];let length=0;
 try {for(;;){const {done,value}=await reader.read();if(done)break;length+=value.length;if(length>limit){await reader.cancel();fail('body_too_large',413);}chunks.push(value);}}finally{reader.releaseLock();}
 const out=new Uint8Array(length);let at=0;for(const c of chunks){out.set(c,at);at+=c.length;}return out;
}
export function cookie(request,name) {return request.headers.get('Cookie')?.split(';').map(s=>s.trim()).find(s=>s.startsWith(name+'='))?.slice(name.length+1) || '';}
export const setCookie=(name,value,age)=>`${name}=${value}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=${age}`;
export async function session(request,env,required=true) {
 const token=cookie(request,'__Host-apparent_session');
 const row=token && await stmt(env,'SELECT s.*,a.email,a.beta,a.interest FROM sessions s JOIN accounts a ON a.id=s.account_id WHERE s.hash=? AND s.expires_at>?',await hash(token),Date.now()).first();
 if(!row&&required)fail('unauthorized',401);return row||null;
}
export function csrf(request,env,s) {if(request.headers.get('Origin')!==env.SITE_ORIGIN || request.headers.get('X-CSRF-Token')!==s.csrf)fail('csrf_failed',403);}
export async function bearer(request,env,write=false) {
 const token=/^Bearer ([a-f0-9]{64})$/.exec(request.headers.get('Authorization')||'')?.[1];if(!token)fail('unauthorized',401);
 const row=await stmt(env,'SELECT t.*,a.beta FROM tokens t JOIN accounts a ON a.id=t.account_id WHERE t.hash=? AND t.expires_at>?',await hash(token),Date.now()).first();
 if(!row)fail('unauthorized',401);if(row.beta!=='active')fail('beta_required',403);if(write&&row.scope!=='upload')fail('forbidden',403);return row;
}
export async function usage(env,account) {return (await stmt(env,"SELECT COALESCE(sum(bytes),0) AS bytes FROM captures WHERE account_id=? AND status!='deleted' AND expires_at>? AND (status='ready' OR pending_until>?)",account,Date.now(),Date.now()).first()).bytes;}
export async function slots(env) {const {used}=await stmt(env,"SELECT count(*) AS used FROM accounts WHERE beta='active'").first();return {used,limit:20,remaining:Math.max(0,20-used)};}
export async function rate(env,key,max=30) {
 const window=Math.floor(Date.now()/3600000);
 const row=await stmt(env,'INSERT INTO rate_limits(key,window,count) VALUES(?,?,1) ON CONFLICT(key) DO UPDATE SET count=CASE WHEN window=excluded.window THEN count+1 ELSE 1 END,window=excluded.window RETURNING count',key,window).first();
 if(row.count>max)fail('slow_down',429);
}
export async function envelope(env,value,decrypt=false) {
 const bytes=Uint8Array.from(atob(env.TOKEN_ENCRYPTION_KEY||''),c=>c.charCodeAt(0));if(bytes.length!==32)throw new Error('key configuration');
 const key=await crypto.subtle.importKey('raw',bytes,'AES-GCM',false,['encrypt','decrypt']);
 if(decrypt){const b=Uint8Array.from(atob(value),c=>c.charCodeAt(0));return new TextDecoder().decode(await crypto.subtle.decrypt({name:'AES-GCM',iv:b.slice(0,12)},key,b.slice(12)));}
 const iv=crypto.getRandomValues(new Uint8Array(12)),cipher=new Uint8Array(await crypto.subtle.encrypt({name:'AES-GCM',iv},key,new TextEncoder().encode(value)));
 return btoa(String.fromCharCode(...iv,...cipher));
}
