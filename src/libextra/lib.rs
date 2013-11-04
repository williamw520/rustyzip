#[link(name = "rustyzip",
       vers = "0.9",
       uuid = "f5bd100e-dbda-4e45-a461-493bd6da5b38")];
#[crate_type = "lib"];


#[deny(non_camel_case_types)];
#[deny(missing_doc)];



/******************************************************************************
 * RustyZip, compression library in Rust.
 */


/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: RustyZip
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/


/// The modules in this crate
// make mod pub so that its pub names can be linked by the linker.
pub mod deflate;
pub mod gzip;
