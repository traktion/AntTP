use crate::service::command_service::CommandService;
use crate::service::chunk_service::ChunkService;
use crate::service::public_data_service::PublicDataService;
use crate::service::pnr_service::PnrService;
use actix_web::web::Data;
use ant_evm::EvmWallet;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool_handler, ServerHandler};


    pub mod pnr_tool;
    pub mod chunk_tool;
    pub mod command_tool;
    pub mod public_data_tool;

    #[derive(Debug, Clone)]
    pub struct McpTool {
        command_service: Data<CommandService>,
        chunk_service: Data<ChunkService>,
        pnr_service: Data<PnrService>,
        public_data_service: Data<PublicDataService>,
        evm_wallet: Data<EvmWallet>,
        tool_router: ToolRouter<Self>,
    }

    impl McpTool {
        pub fn new(
            command_service: Data<CommandService>,
            chunk_service: Data<ChunkService>,
            pnr_service: Data<PnrService>,
            public_data_service: Data<PublicDataService>,
            evm_wallet: Data<EvmWallet>
        ) -> Self {
            Self {
                command_service,
                chunk_service,
                pnr_service,
                public_data_service,
                evm_wallet,
                tool_router: Self::chunk_tool_router()
                    + Self::pnr_tool_router()
                    + Self::command_tool_router()
                    + Self::public_data_tool_router()
            }
        }
    }

#[tool_handler]
impl ServerHandler for McpTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("AntTP tools for creating and retrieving data on Autonomi Network".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}