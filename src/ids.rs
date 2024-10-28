//! IDs of the resources defined in "resources\example03.res".
//!
//! They are the "glue" between our Rust code and the dialog resources.

use winsafe::seq_ids;

seq_ids! {
	DLG_CREATE = 1000;
	LBL_OLD
	TXT_OLD
	BTN_OLD
	LBL_NEW
	TXT_NEW
	BTN_NEW
	TXT_LVL
	TRACK_LVL
	BTN_CREATE
	TXT_CREATE
}

seq_ids! {
	DLG_APPLY = 2000;
	LBL_PATH
	TXT_PATH
	BTN_PATH
	LBL_PATCH
	TXT_PATCH
	BTN_PATCH
	BTN_APPLY
	TXT_APPLY
}