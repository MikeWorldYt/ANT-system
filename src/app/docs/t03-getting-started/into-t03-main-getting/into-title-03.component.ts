import { Component, Input, } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { CommonModule } from '@angular/common';
// ▲ COMPONENTS ▲
import { FooterComponent } from '../../../layout/footer/footer.component';

// ▲ SERVICES ▲
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';

// ▲ FEATURES ▲
import { ContentPageComponent } from '../../content-page/content-page.component';

@Component({
  selector: 'docs-t03-into-nomenclature',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    FooterComponent,
    ContentPageComponent
  ],
  templateUrl: './into-title-03.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T03_IntoComponent {
  // @Input() path: [string, string, string] = ['EN', 'title_02', 'page_01'];

  // ▲ SERVICES ▲
  constructor(
    private titleService: TitleService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = 'EN';
  currentTitle: string = 'title_03';
  currentPage: string = 'page_01';

  // ███ OnInit ███ 
  ngOnInit(): void {
    this.titleService.setTitle(this.currentTitle);
    this.pageService.setCurrentPage(this.currentPage);
  }

}