import { renderToString } from "react-dom/server"
import { App } from "./App"
import { Hosted } from "./hosted/Hosted"

export const render = () => renderToString(<App />)
export const renderHosted = (devicePage: boolean) => renderToString(<Hosted devicePage={devicePage} />)
export { mapPage } from "./map/page"
